use std::io::Read;
use async_trait::async_trait;
use crate::LoadAssetError;
use crate::path::AssetPath;

#[async_trait]
pub trait AssetSource: Sync {
    async fn open_asset_file(&self, path: &AssetPath) -> Result<Box<dyn AssetReader>, LoadAssetError>;
}

#[async_trait]
pub trait AssetReader: Send {
    async fn read_fully(&mut self) -> Vec<u8>;
}

pub struct ReadAssetReader<R: Read + Send> {
    read: R,
}

impl<R: Read + Send> ReadAssetReader<R> {
    fn new(read: R) -> Self {
        ReadAssetReader { read }
    }
}

#[async_trait]
impl<R: Read + Send> AssetReader for ReadAssetReader<R> {
    async fn read_fully(&mut self) -> Vec<u8> {
        let mut vec = Vec::new();
        self.read.read_to_end(&mut vec).unwrap();
        vec
    }
}

pub mod desktop_fs {
    use std::fs::File;
    use std::path::PathBuf;
    use async_trait::async_trait;
    use crate::LoadAssetError;
    use crate::path::AssetPath;
    use crate::source::{AssetReader, AssetSource, ReadAssetReader};

    pub struct DirectoryAssetSource {
        directory: PathBuf,
    }

    impl DirectoryAssetSource {
        pub fn new<P: Into<PathBuf>>(path: P) -> Self {
            // TODO: Validate
            DirectoryAssetSource { directory: path.into() }
        }
    }

    #[async_trait]
    impl AssetSource for DirectoryAssetSource {
        async fn open_asset_file(&self, path: &AssetPath) -> Result<Box<dyn AssetReader>, LoadAssetError> {
            let file_path = path.path_string()
                .trim_start_matches("/")
                .split("/")
                .fold(self.directory.clone(), |path, segment| path.join(segment));

            match File::open(file_path) {
                Err(_) => Err(LoadAssetError::NotFound(path.clone())),
                Ok(file) => Ok(Box::new(ReadAssetReader::new(file))),
            }
        }
    }
}
