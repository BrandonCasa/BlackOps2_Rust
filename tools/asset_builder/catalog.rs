use crate::{core::*, fastfile::Address, native::Zone};
use std::collections::BTreeMap;

pub struct Catalog<'a> {
    pub zones: &'a [(String, Zone)],
    definitions: BTreeMap<(u32, String), (usize, Address, String)>,
}

impl<'a> Catalog<'a> {
    pub fn definitions(&self, kind: u32) -> impl Iterator<Item = (&str, usize, Address)> {
        self.definitions
            .iter()
            .filter(move |((asset_kind, _), _)| *asset_kind == kind)
            .map(|(_, (zone, root, name))| (name.as_str(), *zone, *root))
    }

    pub fn pointer(
        &self,
        zone: usize,
        field: Address,
        kind: u32,
    ) -> Result<(String, usize, Address)> {
        let streams = &self.zones[zone].1.streams;
        let root = Address::decode(streams.u32(field)?)?;
        let name_offset = if kind == 8 { 72 } else { 0 };
        let name = streams.text(Address::decode(streams.u32(root.add(name_offset)?)?)?)?;
        let (owner, definition) = self.resolve(kind, &name)?;
        Ok((name.trim_start_matches(',').to_owned(), owner, definition))
    }

    pub fn new(zones: &'a [(String, Zone)]) -> Self {
        let mut definitions = BTreeMap::new();
        for (index, (_, zone)) in zones.iter().enumerate() {
            for asset in zone.assets.iter().filter(|asset| !asset.reference) {
                definitions.insert(
                    (asset.kind, asset.name.to_ascii_lowercase()),
                    (index, asset.root, asset.name.clone()),
                );
            }
        }
        Self { zones, definitions }
    }

    pub fn resolve(&self, kind: u32, name: &str) -> Result<(usize, Address)> {
        let key = (kind, name.trim_start_matches(',').to_ascii_lowercase());
        if let Some(&(zone, root, _)) = self.definitions.get(&key) {
            return Ok((zone, root));
        }
        if kind == 6
            && let Some(&(zone, root, _)) = self.definitions.get(&(kind, "$default".into()))
        {
            println!("Resolved native material {name} through the game's $default fallback");
            return Ok((zone, root));
        }
        Err(format!("Native asset definition is missing: type {kind} {name}").into())
    }
}
