use crate::{convert::Catalog, core::*, fastfile::Address};
use std::collections::BTreeMap;

pub type Entity = BTreeMap<String, String>;

fn token(bytes: &[u8], cursor: &mut usize) -> Result<Option<String>> {
    loop {
        while bytes.get(*cursor).is_some_and(|b| b.is_ascii_whitespace() || *b == 0) { *cursor += 1; }
        if bytes.get(*cursor..*cursor + 2) == Some(b"//") {
            while bytes.get(*cursor).is_some_and(|b| *b != b'\n') { *cursor += 1; }
        } else { break; }
    }
    let Some(&first) = bytes.get(*cursor) else { return Ok(None); };
    *cursor += 1;
    if matches!(first, b'{' | b'}') { return Ok(Some((first as char).to_string())); }
    let mut value = Vec::new();
    if first == b'"' {
        loop {
            let byte = *bytes.get(*cursor).ok_or("Unterminated native entity token")?;
            *cursor += 1;
            if byte == b'"' { break; }
            if byte == b'\\' && bytes.get(*cursor).is_some_and(|b| matches!(b, b'"' | b'\\')) {
                value.push(bytes[*cursor]); *cursor += 1;
            } else { value.push(byte); }
        }
    } else {
        value.push(first);
        while let Some(&byte) = bytes.get(*cursor) {
            if byte.is_ascii_whitespace() || matches!(byte, 0 | b'{' | b'}') { break; }
            value.push(byte); *cursor += 1;
        }
    }
    Ok(Some(String::from_utf8(value)?))
}

pub fn read(catalog: &Catalog<'_>) -> Result<Vec<Entity>> {
    let mut entities = Vec::new();
    for (_, zone, root) in catalog.definitions(16) {
        let streams = &catalog.zones[zone].1.streams;
        let length = streams.u32(root.add(8)?)? as usize;
        let bytes = streams.read(Address::decode(streams.u32(root.add(4)?)?)?, length)?;
        let mut cursor = 0;
        while let Some(start) = token(bytes, &mut cursor)? {
            if start != "{" { return Err("Native entity record must start with an opening brace".into()); }
            let mut entity = Entity::new();
            loop {
                let key = token(bytes, &mut cursor)?.ok_or("Unterminated native entity record")?;
                if key == "}" { break; }
                if key == "{" { return Err("Nested native entity record".into()); }
                let value = token(bytes, &mut cursor)?.ok_or("Native entity key has no value")?;
                if matches!(value.as_str(), "{" | "}") { return Err("Native entity key has no value".into()); }
                if entity.insert(key.clone(), value).is_some() { return Err(format!("Duplicate native entity key {key}").into()); }
            }
            entities.push(entity);
        }
    }
    let keys: std::collections::BTreeSet<_> = entities.iter().flat_map(|entity| entity.keys()).collect();
    let brushes = entities.iter().filter(|entity| entity.get("model").is_some_and(|name| name.starts_with('*'))).count();
    println!("Read {} native map entities, {} distinct fields, {brushes} brush placements", entities.len(), keys.len());
    Ok(entities)
}

pub fn vector(entity: &Entity, key: &str) -> Result<Option<[f32; 3]>> {
    let Some(value) = entity.get(key) else { return Ok(None); };
    let components = value.split_whitespace().map(str::parse).collect::<std::result::Result<Vec<f32>, _>>()?;
    let result: [f32; 3] = components.try_into().map_err(|_| format!("Native entity {key} is not a three-component vector"))?;
    if !result.iter().all(|v| v.is_finite()) { return Err(format!("Non-finite native entity {key}").into()); }
    Ok(Some(result))
}
