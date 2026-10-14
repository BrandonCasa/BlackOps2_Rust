use crate::{
    catalog::Catalog,
    core::*,
    fastfile::{Address, Streams},
    pack::Pack,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
    sync::Arc,
};

struct Bank {
    file: File,
    entries: BTreeMap<u32, [u8; 20]>,
    name: String,
    streamed: bool,
}

fn text(streams: &Streams, field: Address) -> Result<String> {
    let pointer = streams.u32(field)?;
    if pointer == 0 {
        return Ok(String::new());
    }
    Ok(streams.text(Address::decode(pointer)?)?)
}

pub(crate) fn alias_hashes(catalog: &Catalog<'_>) -> Result<BTreeMap<u32, String>> {
    let mut names = BTreeMap::<u32, String>::new();
    for (_, zone) in catalog.zones {
        let streams = &zone.streams;
        for asset in zone
            .assets
            .iter()
            .filter(|asset| asset.kind == 9 && !asset.reference)
        {
            let count = streams.u32(asset.root.add(4)?)? as usize;
            if count == 0 {
                continue;
            }
            let lists = Address::decode(streams.u32(asset.root.add(8)?)?)?;
            for index in 0..count {
                let list = lists.add(index * 20)?;
                let hash = streams.u32(list.add(4)?)?;
                let name = text(streams, list)?;
                if hash == 0 || name.is_empty() {
                    return Err("Native sound alias list lacks its ID or name".into());
                }
                if names
                    .get(&hash)
                    .is_some_and(|existing| !existing.eq_ignore_ascii_case(&name))
                {
                    return Err(format!(
                        "Native sound alias ID {hash:#010x} resolves conflicting names"
                    )
                    .into());
                }
                names.insert(hash, name);
            }
        }
    }
    Ok(names)
}

pub(crate) fn definitions(
    catalog: &Catalog<'_>,
    required: &std::collections::BTreeSet<String>,
    missing: &mut Vec<String>,
) -> Result<Vec<Value>> {
    let mut lists = BTreeMap::new();
    for (zone_index, (_, zone)) in catalog.zones.iter().enumerate() {
        let streams = &zone.streams;
        for asset in zone
            .assets
            .iter()
            .filter(|asset| asset.kind == 9 && !asset.reference)
        {
            let count = streams.u32(asset.root.add(4)?)? as usize;
            if count == 0 {
                continue;
            }
            let lists_address = Address::decode(streams.u32(asset.root.add(8)?)?)?;
            for index in 0..count {
                let list = lists_address.add(index * 20)?;
                lists.insert(
                    text(streams, list)?.to_ascii_lowercase(),
                    (zone_index, list),
                );
            }
        }
    }
    let mut pending: Vec<_> = required
        .iter()
        .map(|name| {
            (
                name.to_ascii_lowercase(),
                "selected initial consumer".to_owned(),
            )
        })
        .collect();
    let mut selected = std::collections::BTreeSet::new();
    let mut rows = Vec::new();
    while let Some((name, consumer)) = pending.pop() {
        if !selected.insert(name.clone()) {
            continue;
        }
        let Some(&(zone, list)) = lists.get(&name) else {
            missing.push(format!("Initial sound alias {name} referenced by {consumer} has no installed native bank definition"));
            continue;
        };
        let streams = &catalog.zones[zone].1.streams;
        let count = streams.u32(list.add(12)?)? as usize;
        if count == 0 {
            return Err(format!("Required native sound alias {name} has no variants").into());
        }
        let aliases = Address::decode(streams.u32(list.add(8)?)?)?;
        for index in 0..count {
            let row = definition(streams, &name, index, list, aliases.add(index * 96)?)?;
            if let Some(secondary) = row["secondary"].as_str().filter(|name| !name.is_empty()) {
                pending.push((
                    secondary.to_ascii_lowercase(),
                    format!("secondaryName of {name}/{index}"),
                ));
            }
            rows.push(row);
        }
    }
    Ok(rows)
}

fn definition(
    streams: &Streams,
    name: &str,
    index: usize,
    list: Address,
    alias: Address,
) -> Result<Value> {
    Ok(
        json!({"name":name,"index":index,"asset_id":streams.u32(alias.add(16)?)?,
        "flags":streams.u32(alias.add(24)?)?,"flags1":streams.u32(alias.add(28)?)?,
        "list_sequence":streams.u32(list.add(16)?)?,"context_type":streams.u32(alias.add(36)?)?,
        "context_value":streams.u32(alias.add(40)?)?,"probability":streams.u8(alias.add(88)?)?,
        "start_delay":streams.u16(alias.add(54)?)?,"subtitle":text(streams,alias.add(8)?)?,
        "secondary":text(streams,alias.add(12)?)?,"parameters":streams.read(alias.add(28)?,68)?}),
    )
}

impl Bank {
    fn open(path: &Path, streamed: bool) -> Result<Self> {
        let mut file = File::open(path)?;
        let length = file.metadata()?.len();
        let mut header = [0; 2048];
        file.read_exact(&mut header)?;
        if u32_at(&header, 0) != 0x2358_5532
            || u32_at(&header, 4) != 14
            || u32_at(&header, 8) != 20
            || u32_at(&header, 12) != 16
            || u32_at(&header, 16) != 64
            || u64_at(&header, 32) != length
        {
            return Err(format!("Invalid native sound bank header: {}", path.display()).into());
        }
        let count = u32_at(&header, 20) as usize;
        let offset = u64_at(&header, 40);
        if offset
            .checked_add(count as u64 * 20)
            .is_none_or(|end| end > length)
        {
            return Err("Native sound TOC exceeds bank".into());
        }
        file.seek(SeekFrom::Start(offset))?;
        let mut entries = BTreeMap::new();
        for _ in 0..count {
            let mut entry = [0; 20];
            file.read_exact(&mut entry)?;
            if u32_at(&entry, 8) as u64 + u32_at(&entry, 4) as u64 > length {
                return Err("Native sound payload exceeds bank".into());
            }
            if entries.insert(u32_at(&entry, 0), entry).is_some() {
                return Err("Duplicate native sound asset ID in bank".into());
            }
        }
        Ok(Self {
            file,
            entries,
            name: path.file_name().unwrap().to_string_lossy().into(),
            streamed,
        })
    }

    fn read(&mut self, entry: &[u8; 20]) -> Result<Vec<u8>> {
        self.file.seek(SeekFrom::Start(u32_at(entry, 8) as u64))?;
        let mut bytes = vec![0; u32_at(entry, 4) as usize];
        self.file.read_exact(&mut bytes)?;
        Ok(bytes)
    }
}

fn encode(entry: &[u8], payload: Vec<u8>) -> Result<(Vec<u8>, &'static str, u32)> {
    let rates = [
        8000_u32, 12000, 16000, 24000, 32000, 44100, 48000, 96000, 192000,
    ];
    let rate = *rates
        .get(entry[16] as usize)
        .ok_or("Invalid sound frame-rate index")?;
    let channels = entry[17] as u16;
    let frames = u32_at(entry, 12);
    if channels == 0 || channels > 2 || frames == 0 {
        return Err(format!("Invalid native audio dimensions: {channels} channels, {frames} frames, entry {entry:02x?}").into());
    }
    match entry[19] {
        0 => {
            if payload.len() as u64 != frames as u64 * channels as u64 * 2 {
                return Err("PCM16 payload disagrees with native frame count".into());
            }
            let mut bytes = Vec::with_capacity(44 + payload.len());
            bytes.extend(b"RIFF");
            push32(&mut bytes, payload.len() as u32 + 36);
            bytes.extend(b"WAVEfmt ");
            push32(&mut bytes, 16);
            push16(&mut bytes, 1);
            push16(&mut bytes, channels);
            push32(&mut bytes, rate);
            push32(&mut bytes, rate * channels as u32 * 2);
            push16(&mut bytes, channels * 2);
            push16(&mut bytes, 16);
            bytes.extend(b"data");
            push32(&mut bytes, payload.len() as u32);
            bytes.extend(payload);
            Ok((bytes, "wav", rate))
        }
        8 => {
            if payload.get(..4) != Some(b"fLaC")
                || payload.len() < 42
                || payload[4] & 127 != 0
                || payload[5..8] != [0, 0, 34]
            {
                return Err("Native FLAC lacks STREAMINFO".into());
            }
            let info = u64::from_be_bytes(payload[18..26].try_into()?);
            if info >> 44 != rate as u64
                || (info >> 41 & 7) + 1 != channels as u64
                || (info >> 36 & 31) + 1 != 16
                || info & 0x0f_ffff_ffff != frames as u64
            {
                return Err("FLAC STREAMINFO disagrees with native bank entry".into());
            }
            Ok((payload, "flac", rate))
        }
        format => Err(format!("Unsupported native sound format {format}").into()),
    }
}

pub fn convert(
    install: &Path,
    catalog: &Catalog<'_>,
    required: Option<&std::collections::BTreeSet<String>>,
    pack: &mut Pack,
) -> Result<Vec<Value>> {
    let mut banks = Vec::new();
    let mut resident = BTreeMap::new();
    let mut lists = BTreeMap::new();
    let mut seen_banks = std::collections::BTreeSet::new();
    for (zone_index, (_, zone)) in catalog.zones.iter().enumerate() {
        let streams = &zone.streams;
        for asset in zone
            .assets
            .iter()
            .filter(|asset| asset.kind == 9 && !asset.reference)
        {
            for (offset, streamed, extension) in [(0x20, true, "sabs"), (0x942, false, "sabl")] {
                let name = text(streams, asset.root.add(offset)?)?;
                let language = text(streams, asset.root.add(offset + 4)?)?;
                if name.is_empty() || language.is_empty() {
                    continue;
                }
                let filename = format!("{name}.{language}.{extension}");
                if !seen_banks.insert(filename.clone()) {
                    continue;
                }
                let path = install.join("sound").join(&filename);
                if path.is_file() {
                    let bank = Bank::open(&path, streamed)?;
                    println!(
                        "Read {filename}: {} native sound entries",
                        bank.entries.len()
                    );
                    banks.push(bank);
                } else {
                    println!(
                        "Native bank {filename} is absent; required alias payloads will be checked individually"
                    );
                }
            }
            let count = streams.u32(asset.root.add(0x1270)?)? as usize;
            if count != 0 {
                let entries = Address::decode(streams.u32(asset.root.add(0x1274)?)?)?;
                let data = Address::decode(streams.u32(asset.root.add(0x127c)?)?)?;
                let size = streams.u32(asset.root.add(0x1278)?)? as usize;
                for index in 0..count {
                    let entry = streams.read(entries.add(index * 20)?, 20)?;
                    if u32_at(entry, 0) == 0 {
                        continue;
                    }
                    let offset = u32_at(entry, 8) as usize;
                    if offset == u32::MAX as usize {
                        continue;
                    }
                    let length = u32_at(entry, 4) as usize;
                    if offset.checked_add(length).is_none_or(|end| end > size) {
                        return Err("Resident sound exceeds loaded allocation".into());
                    }
                    resident.entry(u32_at(entry, 0)).or_insert((
                        zone_index,
                        data.add(offset)?,
                        entry.to_vec(),
                    ));
                }
            }
            let count = streams.u32(asset.root.add(4)?)? as usize;
            if count == 0 {
                continue;
            }
            let array = Address::decode(streams.u32(asset.root.add(8)?)?)?;
            for index in 0..count {
                let list = array.add(index * 20)?;
                lists.insert(text(streams, list)?, (zone_index, list));
            }
        }
    }
    let selected = if let Some(required) = required {
        let index: BTreeMap<_, _> = lists
            .iter()
            .map(|(name, binding)| (name.to_ascii_lowercase(), *binding))
            .collect();
        let mut selected = std::collections::BTreeSet::new();
        let mut pending: Vec<_> = required.iter().cloned().collect();
        while let Some(name) = pending.pop() {
            let name = name.to_ascii_lowercase();
            if !selected.insert(name.clone()) {
                continue;
            }
            let &(zone, list) = index
                .get(&name)
                .ok_or_else(|| format!("Required native sound alias {name} is missing"))?;
            let streams = &catalog.zones[zone].1.streams;
            let aliases = Address::decode(streams.u32(list.add(8)?)?)?;
            for variant in 0..streams.u32(list.add(12)?)? as usize {
                let secondary = text(streams, aliases.add(variant * 96 + 12)?)?;
                if !secondary.is_empty() {
                    pending.push(secondary);
                }
            }
        }
        Some(selected)
    } else {
        None
    };
    let mut rows = Vec::new();
    let mut converted = BTreeMap::new();
    for (name, (zone_index, list)) in lists {
        if selected
            .as_ref()
            .is_some_and(|selected| !selected.contains(&name.to_ascii_lowercase()))
        {
            continue;
        }
        let (zone_name, zone) = &catalog.zones[zone_index];
        let streams = &zone.streams;
        let count = streams.u32(list.add(12)?)? as usize;
        if count == 0 {
            return Err(format!("Native sound alias {name} has no variants").into());
        }
        let aliases = Address::decode(streams.u32(list.add(8)?)?)?;
        for index in 0..count {
            let alias = aliases.add(index * 96)?;
            let id = streams.u32(alias.add(16)?)?;
            let flags = streams.u32(alias.add(24)?)?;
            let streamed = (flags >> 15 & 3) >= 2;
            let language = if zone_name.starts_with("en_") {
                "english"
            } else {
                "all"
            };
            let mut row = definition(streams, &name, index, list, alias)?;
            row["zone"] = json!(format!("{language}-{zone_name}"));
            if id == 0 {
                row["silent"] = json!(true);
                rows.push(row);
                continue;
            }
            let key = (id, streamed);
            let binding = if let Some(binding) = converted.get(&key) {
                binding
            } else {
                let (entry, payload, source) = if !streamed
                    && let Some((owner, data, entry)) = resident.get(&id)
                {
                    (
                        entry.clone(),
                        catalog.zones[*owner]
                            .1
                            .streams
                            .read(*data, u32_at(entry, 4) as usize)?
                            .to_vec(),
                        "resident".to_owned(),
                    )
                } else if let Some(bank) = banks
                    .iter_mut()
                    .find(|bank| bank.streamed == streamed && bank.entries.contains_key(&id))
                {
                    let entry = bank.entries[&id];
                    (entry.to_vec(), bank.read(&entry)?, bank.name.clone())
                } else {
                    return Err(format!("Required sound payload is missing: alias {name}/{index}, asset {id:08x}, file {}, load type {}", text(streams, alias.add(20)?)?, flags >> 15 & 3).into());
                };
                let (bytes, extension, rate) = encode(&entry, payload)
                    .map_err(|error| format!("Sound {name}/{index} from {source}: {error}"))?;
                let uri = format!(
                    "prepared/sounds/{}/{id:08x}.{extension}",
                    if streamed { "streamed" } else { "resident" }
                );
                pack.entries.insert(uri.clone(), Arc::new(bytes));
                converted.insert(key, (uri, u32_at(&entry, 12) as f64 / rate as f64));
                &converted[&key]
            };
            row["uri"] = json!(binding.0);
            row["seconds"] = json!(binding.1);
            rows.push(row);
        }
    }
    println!(
        "Converted {} sound alias variants, {} distinct payloads",
        rows.len(),
        converted.len()
    );
    Ok(rows)
}
