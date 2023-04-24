use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::error::Error;
use async_trait::async_trait;
use thiserror::Error;
use crate::path::{AssetPath, InvalidCharacters};
use crate::source::AssetSource;

pub mod path;
pub mod source;

#[async_trait(? Send)]
pub trait AssetPipeline {
    async fn load_asset(&self, path: AssetPath, source: &dyn AssetSource) -> Result<Box<dyn Any>, LoadAssetError>;
}

pub struct AssetPipelines {
    pipelines: HashMap<TypeId, Box<dyn AssetPipeline>>,
}

#[derive(Debug, Error)]
pub enum LoadAssetError {
    #[error("unknown asset type {:?}", .0)]
    UnknownType(TypeId),
    #[error("could not find asset: {:?}", .0)]
    NotFound(AssetPath),
    #[error("{}", .0)]
    InvalidPath(InvalidCharacters),
    #[error("unknown error loading asset: {:?}", .0)]
    UnknownError(AssetPath),
    #[error("pipeline error: {}", .0)]
    Other(Box<dyn Error>),
}

impl LoadAssetError {
    pub fn other<T: Error + 'static>(err: T) -> LoadAssetError {
        LoadAssetError::Other(Box::new(err))
    }
}

impl AssetPipelines {
    pub fn new(pipelines: HashMap<TypeId, Box<dyn AssetPipeline>>) -> Self {
        AssetPipelines { pipelines }
    }

    pub async fn load_asset_of_type(&self, path: AssetPath, typ: TypeId, source: &impl AssetSource) -> Result<Box<dyn Any>, LoadAssetError> {
        let pipeline = self.pipelines.get(&typ)
            .ok_or_else(|| LoadAssetError::UnknownType(typ))?;
        pipeline.load_asset(path, source).await
    }

    pub async fn load_asset<T: 'static>(&self, path: AssetPath, source: &impl AssetSource) -> Result<T, LoadAssetError> {
        let boxed = self.load_asset_of_type(path.clone(), TypeId::of::<T>(), source).await?;
        Ok(*boxed.downcast::<T>().unwrap())
    }
}


