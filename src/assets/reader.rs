use bevy::asset::io::{AssetReader, AssetReaderError, PathStream, Reader, VecReader};
use std::{path::Path, sync::Arc};

pub(crate) struct PackAssetReader(pub Option<Arc<crate::pack::PackIndex>>);

impl AssetReader for PackAssetReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let name = path.to_str().ok_or_else(|| AssetReaderError::NotFound(path.to_owned()))?.replace('\\', "/");
        let index = self.0.as_ref().ok_or_else(|| AssetReaderError::NotFound(path.to_owned()))?;
        Ok(VecReader::new(index.get(&name).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                AssetReaderError::NotFound(path.to_owned())
            } else { error.into() }
        })?))
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        Err::<VecReader, _>(AssetReaderError::NotFound(path.to_owned()))
    }

    async fn read_directory<'a>(&'a self, path: &'a Path) -> Result<Box<PathStream>, AssetReaderError> {
        let index = self.0.as_ref().ok_or_else(|| AssetReaderError::NotFound(path.to_owned()))?;
        let paths: std::collections::BTreeSet<_> = index.entries.keys().filter_map(|name| {
            let remainder = Path::new(name).strip_prefix(path).ok()?;
            Some(path.join(remainder.components().next()?.as_os_str()))
        }).collect();
        Ok(Box::new(bevy::tasks::futures_lite::stream::iter(paths)))
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        Ok(self.0.as_ref().is_some_and(|index| index.entries.keys().any(|name| {
            Path::new(name).strip_prefix(path).is_ok_and(|remainder| remainder.components().next().is_some())
        })))
    }
}
