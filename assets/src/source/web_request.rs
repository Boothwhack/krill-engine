use std::mem::swap;
use async_trait::async_trait;
use reqwest::StatusCode;
use crate::LoadAssetError;
use crate::path::AssetPath;
use crate::source::{AssetReader, AssetSource};

pub struct WebRequestAssetSource {}

#[async_trait(?Send)]
impl AssetSource for WebRequestAssetSource {
    async fn open_asset_file(&self, path: &AssetPath) -> Result<Box<dyn AssetReader>, LoadAssetError> {
        let url = path.path_string().trim_start_matches("/");
        match reqwest::get(url).await {
            Ok(response) => match response.status() {
                StatusCode::OK => {
                    let response = response.bytes().await.map_err(|_| LoadAssetError::UnknownError(path.clone()))?.to_vec();
                    Ok(Box::new(WebRequestAssetReader { response }) as _)
                },
                StatusCode::NOT_FOUND => Err(LoadAssetError::NotFound(path.clone())),
                _ => Err(LoadAssetError::UnknownError(path.clone())),
            }
            Err(err) => Err(LoadAssetError::other(err)),
        }
    }
}

struct WebRequestAssetReader {
    response: Vec<u8>,
}

#[async_trait]
impl AssetReader for WebRequestAssetReader {
    async fn read_fully(&mut self) -> Vec<u8> {
        let mut response = Vec::new();
        swap(&mut self.response, &mut response);
        response
    }
}
