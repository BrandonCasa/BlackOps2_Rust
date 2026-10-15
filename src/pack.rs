use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

const MAGIC: &[u8; 8] = b"ORIGPAK1";

#[derive(Clone, Debug, Default)]
pub struct Pack {
    pub entries: BTreeMap<String, Arc<Vec<u8>>>,
    stream: Option<Arc<Mutex<Stream>>>,
}

#[derive(Debug)]
struct Stream {
    path: PathBuf,
    file: std::fs::File,
    names: BTreeSet<String>,
    finished: bool,
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
        if &magic != MAGIC {
            return Err(invalid("Unsupported runtime pack"));
        }
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
            let end = start
                .checked_add(name_length as u64)
                .and_then(|offset| offset.checked_add(size))
                .filter(|end| *end <= length)
                .ok_or_else(|| invalid("Truncated runtime pack"))?;
            let mut name = vec![0; name_length];
            reader.read_exact(&mut name)?;
            let name = String::from_utf8(name)
                .map_err(|_| invalid("Invalid runtime asset path encoding"))?;
            if !valid_path(&name) {
                return Err(invalid(format!("Invalid runtime asset path: {name}")));
            }
            if entries
                .insert(name.clone(), (start + name_length as u64, size))
                .is_some()
            {
                return Err(invalid(format!("Duplicate runtime asset: {name}")));
            }
            reader.seek(SeekFrom::Start(end))?;
        }
        if reader.stream_position()? != length {
            return Err(invalid("Unexpected trailing runtime pack bytes"));
        }
        Ok(Self {
            path: path.to_owned(),
            entries,
        })
    }

    pub fn get(&self, path: &str) -> io::Result<Vec<u8>> {
        let &(offset, length) = self.entries.get(path).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Runtime pack is missing {path}"),
            )
        })?;
        let length = usize::try_from(length).map_err(|_| invalid("Runtime asset is too large"))?;
        let mut reader = std::fs::File::open(&self.path)?;
        reader.seek(SeekFrom::Start(offset))?;
        let mut bytes = vec![0; length];
        reader.read_exact(&mut bytes)?;
        Ok(bytes)
    }

    pub fn metadata(&self) -> io::Result<Pack> {
        let entries = self
            .entries
            .keys()
            .filter(|path| path.ends_with(".json") || path.ends_with(".collision.gltf"))
            .map(|path| self.get(path).map(|bytes| (path.clone(), Arc::new(bytes))))
            .collect::<io::Result<_>>()?;
        Ok(Pack::from_entries(entries))
    }
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn valid_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', ':', '#', '\0'])
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

impl Pack {
    pub fn from_entries(entries: BTreeMap<String, Arc<Vec<u8>>>) -> Self {
        Self {
            entries,
            stream: None,
        }
    }

    pub fn streaming(path: &Path) -> io::Result<Self> {
        let mut file = std::fs::File::create(path)?;
        file.write_all(MAGIC)?;
        file.write_all(&0_u32.to_le_bytes())?;
        Ok(Self {
            entries: BTreeMap::new(),
            stream: Some(Arc::new(Mutex::new(Stream {
                path: path.to_owned(),
                file,
                names: BTreeSet::new(),
                finished: false,
            }))),
        })
    }

    pub fn contains(&self, path: &str) -> bool {
        self.entries.contains_key(path)
            || self.stream.as_ref().is_some_and(|stream| {
                stream
                    .lock()
                    .is_ok_and(|stream| stream.names.contains(path))
            })
    }

    pub fn store(&mut self, path: String, bytes: Vec<u8>) -> io::Result<()> {
        if !valid_path(&path) {
            return Err(invalid(format!("Invalid runtime asset path: {path}")));
        }
        if self.contains(&path) {
            return Err(invalid(format!("Duplicate runtime asset: {path}")));
        }
        if let Some(stream) = &self.stream {
            let mut stream = stream
                .lock()
                .map_err(|_| invalid("Runtime pack writer failed"))?;
            if stream.finished {
                return Err(invalid("Runtime pack writer already finished"));
            }
            Self::write_entry(&mut stream.file, &path, &bytes)?;
            stream.names.insert(path);
        } else {
            self.entries.insert(path, Arc::new(bytes));
        }
        Ok(())
    }

    fn write_entry(writer: &mut impl Write, path: &str, bytes: &[u8]) -> io::Result<()> {
        let length =
            u32::try_from(path.len()).map_err(|_| invalid("Runtime asset path is too long"))?;
        writer.write_all(&length.to_le_bytes())?;
        writer.write_all(&(bytes.len() as u64).to_le_bytes())?;
        writer.write_all(path.as_bytes())?;
        writer.write_all(bytes)
    }

    pub fn write_to(&self, path: &Path) -> io::Result<()> {
        let Some(stream) = &self.stream else {
            return self.write(io::BufWriter::new(std::fs::File::create(path)?));
        };
        let mut stream = stream
            .lock()
            .map_err(|_| invalid("Runtime pack writer failed"))?;
        if stream.path != path {
            return Err(invalid("Streaming runtime pack output changed"));
        }
        if stream.finished {
            return Err(invalid("Runtime pack writer already finished"));
        }
        let count = u32::try_from(stream.names.len() + self.entries.len())
            .map_err(|_| invalid("Too many runtime assets"))?;
        for (name, bytes) in &self.entries {
            if !valid_path(name) || stream.names.contains(name) {
                return Err(invalid(format!(
                    "Invalid or duplicate runtime asset: {name}"
                )));
            }
            Self::write_entry(&mut stream.file, name, bytes)?;
        }
        stream.file.seek(SeekFrom::Start(8))?;
        stream.file.write_all(&count.to_le_bytes())?;
        stream.file.flush()?;
        stream.finished = true;
        Ok(())
    }

    pub fn write(&self, mut writer: impl Write) -> io::Result<()> {
        if self.stream.is_some() {
            return Err(invalid(
                "Streaming runtime packs must finish at their output path",
            ));
        }
        let count =
            u32::try_from(self.entries.len()).map_err(|_| invalid("Too many runtime assets"))?;
        for path in self.entries.keys() {
            if !valid_path(path) {
                return Err(invalid(format!("Invalid runtime asset path: {path}")));
            }
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
            *remaining = remaining
                .checked_sub(count as u64)
                .ok_or_else(|| invalid("Truncated runtime pack"))?;
            let mut value = vec![0; count];
            reader.read_exact(&mut value)?;
            Ok(value)
        }
        if take(&mut reader, &mut remaining, 8)? != MAGIC {
            return Err(invalid("Unsupported runtime pack"));
        }
        let count =
            u32::from_le_bytes(take(&mut reader, &mut remaining, 4)?.try_into().unwrap()) as usize;
        if count as u64 > remaining / 13 {
            return Err(invalid("Invalid runtime pack entry count"));
        }
        let mut entries = BTreeMap::new();
        for _ in 0..count {
            let path_size =
                u32::from_le_bytes(take(&mut reader, &mut remaining, 4)?.try_into().unwrap())
                    as usize;
            let size = usize::try_from(u64::from_le_bytes(
                take(&mut reader, &mut remaining, 8)?.try_into().unwrap(),
            ))
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
        Ok(Self::from_entries(entries))
    }

    #[cfg(test)]
    fn decode(bytes: &[u8]) -> io::Result<Self> {
        Self::read_from(bytes, bytes.len() as u64)
    }

    pub fn get(&self, path: &str) -> io::Result<&[u8]> {
        self.entries
            .get(path)
            .map(|value| value.as_slice())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Runtime pack is missing {path}"),
                )
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
        assert_eq!(
            pack.get("missing").unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        for end in 0..bytes.len() {
            assert!(Pack::decode(&bytes[..end]).is_err());
        }
    }

    #[test]
    fn indexed_pack_reads_payloads_without_retaining_them() {
        let path =
            std::env::temp_dir().join(format!("origins-pack-index-{}.pack", std::process::id()));
        let pack = Pack::from_entries(BTreeMap::from([
            ("game.json".into(), Arc::new(b"{}".to_vec())),
            (
                "prepared/world.collision.gltf".into(),
                Arc::new(b"collision".to_vec()),
            ),
            ("prepared/image.ktx2".into(), Arc::new(vec![17; 4096])),
        ]));
        let mut bytes = Vec::new();
        pack.write(&mut bytes).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        let index = PackIndex::read(&path).unwrap();
        assert_eq!(index.entries.len(), 3);
        let metadata = index.metadata().unwrap();
        assert_eq!(metadata.entries.len(), 2);
        assert_eq!(index.get("prepared/image.ktx2").unwrap(), vec![17; 4096]);
        assert_eq!(
            index.get("missing").unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        for invalid in [
            entry("../escape", &[]),
            {
                let mut trailing = bytes.clone();
                trailing.push(0);
                trailing
            },
            bytes[..bytes.len() - 1].to_vec(),
        ] {
            std::fs::write(&path, invalid).unwrap();
            assert!(PackIndex::read(&path).is_err());
        }
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn streamed_frames_do_not_stay_in_memory_and_share_the_normal_pack_format() {
        let path =
            std::env::temp_dir().join(format!("origins-streamed-{}.pack", std::process::id()));
        let mut pack = Pack::streaming(&path).unwrap();
        pack.store("prepared/frame.ktx2".into(), vec![27; 1_048_576])
            .unwrap();
        assert!(pack.entries.is_empty());
        assert!(pack.contains("prepared/frame.ktx2"));
        assert!(pack.store("prepared/frame.ktx2".into(), vec![]).is_err());
        pack.entries
            .insert("game.json".into(), Arc::new(b"{}".to_vec()));
        pack.write_to(&path).unwrap();
        assert!(pack.store("prepared/late.ktx2".into(), vec![]).is_err());
        let index = PackIndex::read(&path).unwrap();
        assert_eq!(index.entries.len(), 2);
        assert_eq!(
            index.get("prepared/frame.ktx2").unwrap(),
            vec![27; 1_048_576]
        );
        assert_eq!(Pack::read(&path).unwrap().get("game.json").unwrap(), b"{}");
        drop(pack);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn malformed_names_and_lengths_are_rejected() {
        for path in ["../escape", "/absolute", "a//b", "a\\b", "C:/file", "a/./b"] {
            assert!(Pack::decode(&entry(path, &[])).is_err());
            let pack = Pack::from_entries(BTreeMap::from([(path.into(), Arc::new(Vec::new()))]));
            let mut output = Vec::new();
            assert_eq!(
                pack.write(&mut output).unwrap_err().kind(),
                io::ErrorKind::InvalidData
            );
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
            fn flush(&mut self) -> io::Result<()> {
                self.0.flush()
            }
        }
        let bytes = entry("game.json", b"{\"version\":1}");
        let pack =
            Pack::read_from(Chunked(io::Cursor::new(bytes.clone())), bytes.len() as u64).unwrap();
        let mut output = Chunked(io::Cursor::new(Vec::new()));
        pack.write(&mut output).unwrap();
        assert_eq!(output.0.into_inner(), bytes);
        assert_eq!(
            Pack::read_from(&bytes[..bytes.len() - 1], bytes.len() as u64)
                .unwrap_err()
                .kind(),
            io::ErrorKind::UnexpectedEof
        );
    }
}
