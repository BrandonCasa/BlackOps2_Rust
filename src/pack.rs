use std::{collections::BTreeMap, io::{self, Read, Seek, SeekFrom, Write}, path::{Path, PathBuf}, sync::Arc};

const MAGIC: &[u8; 8] = b"ORIGPAK1";

#[derive(Clone, Debug, Default)]
pub struct Pack {
    pub entries: BTreeMap<String, Arc<Vec<u8>>>,
}

#[derive(Debug)]
pub struct PackIndex {
    path: PathBuf,
    pub entries: BTreeMap<String, (u64, u64)>,
}

impl PackIndex {
    pub fn read(path: &Path) -> io::Result<Self> {
        let mut reader = io::BufReader::new(std::fs::File::open(path)?);
        let length = reader.get_ref().metadata()?.len();
        let mut magic = [0; 8];
        reader.read_exact(&mut magic)?;
        if &magic != MAGIC { return Err(invalid("Unsupported runtime pack")); }
        let mut count = [0; 4];
        reader.read_exact(&mut count)?;
        let count = u32::from_le_bytes(count);
        if u64::from(count) > length.saturating_sub(12) / 13 {
            return Err(invalid("Invalid runtime pack entry count"));
        }
        let mut entries = BTreeMap::new();
        for _ in 0..count {
            let mut header = [0; 12];
            reader.read_exact(&mut header)?;
            let name_length = u32::from_le_bytes(header[..4].try_into().unwrap()) as usize;
            let size = u64::from_le_bytes(header[4..].try_into().unwrap());
            let start = reader.stream_position()?;
            let end = start.checked_add(name_length as u64).and_then(|offset| offset.checked_add(size))
                .filter(|end| *end <= length).ok_or_else(|| invalid("Truncated runtime pack"))?;
            let mut name = vec![0; name_length];
            reader.read_exact(&mut name)?;
            let name = String::from_utf8(name).map_err(|_| invalid("Invalid runtime asset path encoding"))?;
            if !valid_path(&name) { return Err(invalid(format!("Invalid runtime asset path: {name}"))); }
            if entries.insert(name.clone(), (start + name_length as u64, size)).is_some() {
                return Err(invalid(format!("Duplicate runtime asset: {name}")));
            }
            reader.seek(SeekFrom::Start(end))?;
        }
        if reader.stream_position()? != length { return Err(invalid("Unexpected trailing runtime pack bytes")); }
        Ok(Self { path: path.to_owned(), entries })
    }

    pub fn get(&self, path: &str) -> io::Result<Vec<u8>> {
        let &(offset, length) = self.entries.get(path).ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, format!("Runtime pack is missing {path}"))
        })?;
        let length = usize::try_from(length).map_err(|_| invalid("Runtime asset is too large"))?;
        let mut reader = std::fs::File::open(&self.path)?;
        reader.seek(SeekFrom::Start(offset))?;
        let mut bytes = vec![0; length];
        reader.read_exact(&mut bytes)?;
        Ok(bytes)
    }

    pub fn metadata(&self) -> io::Result<Pack> {
        let entries = self.entries.keys().filter(|path| path.ends_with(".json") || path.ends_with(".collision.gltf"))
            .map(|path| self.get(path).map(|bytes| (path.clone(), Arc::new(bytes))))
            .collect::<io::Result<_>>()?;
        Ok(Pack { entries })
    }
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn valid_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', ':', '#', '\0'])
        && path.split('/').all(|part| !part.is_empty() && part != "." && part != "..")
}

impl Pack {
    pub fn write(&self, mut writer: impl Write) -> io::Result<()> {
        let count = u32::try_from(self.entries.len()).map_err(|_| invalid("Too many runtime assets"))?;
        for path in self.entries.keys() {
            if !valid_path(path) { return Err(invalid(format!("Invalid runtime asset path: {path}"))); }
            u32::try_from(path.len()).map_err(|_| invalid("Runtime asset path is too long"))?;
        }
        writer.write_all(MAGIC)?;
        writer.write_all(&count.to_le_bytes())?;
        for (path, value) in &self.entries {
            let length = path.len() as u32;
            writer.write_all(&length.to_le_bytes())?;
            writer.write_all(&(value.len() as u64).to_le_bytes())?;
            writer.write_all(path.as_bytes())?;
            writer.write_all(value.as_slice())?;
        }
        writer.flush()
    }

    pub fn read(path: &Path) -> io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let length = file.metadata()?.len();
        Self::read_from(io::BufReader::new(file), length)
    }

    fn read_from(mut reader: impl Read, mut remaining: u64) -> io::Result<Self> {
        fn take(reader: &mut impl Read, remaining: &mut u64, count: usize) -> io::Result<Vec<u8>> {
            *remaining = remaining.checked_sub(count as u64).ok_or_else(|| invalid("Truncated runtime pack"))?;
            let mut value = vec![0; count];
            reader.read_exact(&mut value)?;
            Ok(value)
        }
        if take(&mut reader, &mut remaining, 8)? != MAGIC {
            return Err(invalid("Unsupported runtime pack"));
        }
        let count = u32::from_le_bytes(take(&mut reader, &mut remaining, 4)?.try_into().unwrap()) as usize;
        if count as u64 > remaining / 13 {
            return Err(invalid("Invalid runtime pack entry count"));
        }
        let mut entries = BTreeMap::new();
        for _ in 0..count {
            let path_size = u32::from_le_bytes(take(&mut reader, &mut remaining, 4)?.try_into().unwrap()) as usize;
            let size = usize::try_from(u64::from_le_bytes(take(&mut reader, &mut remaining, 8)?.try_into().unwrap()))
                .map_err(|_| invalid("Runtime asset is too large"))?;
            let path = String::from_utf8(take(&mut reader, &mut remaining, path_size)?)
                .map_err(|_| invalid("Invalid runtime asset path encoding"))?;
            if !valid_path(&path) {
                return Err(invalid(format!("Invalid runtime asset path: {path}")));
            }
            let value = Arc::new(take(&mut reader, &mut remaining, size)?);
            if entries.insert(path.clone(), value).is_some() {
                return Err(invalid(format!("Duplicate runtime asset: {path}")));
            }
        }
        if remaining != 0 || reader.read(&mut [0])? != 0 {
            return Err(invalid("Unexpected trailing runtime pack bytes"));
        }
        Ok(Self { entries })
    }

    #[cfg(test)]
    fn decode(bytes: &[u8]) -> io::Result<Self> {
        Self::read_from(bytes, bytes.len() as u64)
    }

    pub fn get(&self, path: &str) -> io::Result<&[u8]> {
        self.entries.get(path).map(|value| value.as_slice()).ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, format!("Runtime pack is missing {path}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, value: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::from(MAGIC.as_slice());
        bytes.extend(1_u32.to_le_bytes());
        bytes.extend((path.len() as u32).to_le_bytes());
        bytes.extend((value.len() as u64).to_le_bytes());
        bytes.extend(path.as_bytes());
        bytes.extend(value);
        bytes
    }

    #[test]
    fn runtime_payload_is_read_without_extraction() {
        let bytes = entry("images/test.ktx2", &[1, 2, 3]);
        let pack = Pack::decode(&bytes).unwrap();
        let mut encoded = Vec::new();
        pack.write(&mut encoded).unwrap();
        assert_eq!(encoded, bytes);
        assert_eq!(pack.get("images/test.ktx2").unwrap(), [1, 2, 3]);
        assert_eq!(pack.get("missing").unwrap_err().kind(), io::ErrorKind::NotFound);
        for end in 0..bytes.len() {
            assert!(Pack::decode(&bytes[..end]).is_err());
        }
    }

    #[test]
    fn indexed_pack_reads_payloads_without_retaining_them() {
        let path = std::env::temp_dir().join(format!("origins-pack-index-{}.pack", std::process::id()));
        let pack = Pack { entries: BTreeMap::from([
            ("game.json".into(), Arc::new(b"{}".to_vec())),
            ("prepared/world.collision.gltf".into(), Arc::new(b"collision".to_vec())),
            ("prepared/image.ktx2".into(), Arc::new(vec![17; 4096])),
        ]) };
        let mut bytes = Vec::new();
        pack.write(&mut bytes).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        let index = PackIndex::read(&path).unwrap();
        assert_eq!(index.entries.len(), 3);
        let metadata = index.metadata().unwrap();
        assert_eq!(metadata.entries.len(), 2);
        assert_eq!(index.get("prepared/image.ktx2").unwrap(), vec![17; 4096]);
        assert_eq!(index.get("missing").unwrap_err().kind(), io::ErrorKind::NotFound);
        for invalid in [entry("../escape", &[]), {
            let mut trailing = bytes.clone(); trailing.push(0); trailing
        }, bytes[..bytes.len()-1].to_vec()] {
            std::fs::write(&path, invalid).unwrap();
            assert!(PackIndex::read(&path).is_err());
        }
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn malformed_names_and_lengths_are_rejected() {
        for path in ["../escape", "/absolute", "a//b", "a\\b", "C:/file", "a/./b"] {
            assert!(Pack::decode(&entry(path, &[])).is_err());
            let pack = Pack { entries:BTreeMap::from([(path.into(),Arc::new(Vec::new()))]) };
            let mut output = Vec::new();
            assert_eq!(pack.write(&mut output).unwrap_err().kind(),io::ErrorKind::InvalidData);
            assert!(output.is_empty());
        }
        let mut bytes = entry("game.json", &[]);
        bytes[16..24].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(Pack::decode(&bytes).is_err());
        let mut bytes = entry("game.json", &[]);
        bytes.push(0);
        assert!(Pack::decode(&bytes).is_err());
    }

    #[test]
    fn pack_streams_handle_partial_io_and_actual_eof() {
        struct Chunked(io::Cursor<Vec<u8>>);
        impl Read for Chunked {
            fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
                let length = bytes.len().min(3);
                self.0.read(&mut bytes[..length])
            }
        }
        impl Write for Chunked {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.0.write(&bytes[..bytes.len().min(3)])
            }
            fn flush(&mut self) -> io::Result<()> { self.0.flush() }
        }
        let bytes = entry("game.json", b"{\"version\":1}");
        let pack = Pack::read_from(Chunked(io::Cursor::new(bytes.clone())), bytes.len() as u64).unwrap();
        let mut output = Chunked(io::Cursor::new(Vec::new()));
        pack.write(&mut output).unwrap();
        assert_eq!(output.0.into_inner(), bytes);
        assert_eq!(Pack::read_from(&bytes[..bytes.len()-1], bytes.len() as u64).unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    }
}
