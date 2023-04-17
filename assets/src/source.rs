#[cfg(feature = "desktop_fs")]
pub mod desktop_fs;
#[cfg(feature = "web_request")]
pub mod web_request;

use std::io::Read;
use async_trait::async_trait;
use crate::LoadAssetError;
use crate::path::AssetPath;

#[async_trait(?Send)]
pub trait AssetSource: Sync {
    async fn open_asset_file(&self, path: &AssetPath) -> Result<Box<dyn AssetReader>, LoadAssetError>;
}

#[async_trait(?Send)]
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

#[async_trait(?Send)]
impl<R: Read + Send> AssetReader for ReadAssetReader<R> {
    async fn read_fully(&mut self) -> Vec<u8> {
        let mut vec = Vec::new();
        self.read.read_to_end(&mut vec).unwrap();
        vec
    }
}
