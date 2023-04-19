pub mod serial {
    use std::any::Any;
    use async_trait::async_trait;
    use serde::Deserialize;
    use wgpu::{BindGroupLayoutEntry, BindingType, BufferBindingType};
    use assets::{AssetPipeline, LoadAssetError};
    use assets::path::AssetPath;
    use assets::source::AssetSource;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub struct BindGroupLayoutAsset {
        pub entries: Vec<BindGroupEntryDefinition>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub struct BindGroupEntryDefinition {
        pub visibility: Visibility,
        #[serde(rename = "type")]
        pub ty: EntryType,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum EntryType {
        Buffer {
            #[serde(rename = "type")]
            ty: BufferType,
        },
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum BufferType {
        Uniform,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum Visibility {
        Vertex,
        Fragment,
        VertexAndFragment,
        All,
    }

    pub struct BindGroupAssetPipeline;

    #[async_trait(? Send)]
    impl AssetPipeline for BindGroupAssetPipeline {
        async fn load_asset(&self, path: AssetPath, source: &dyn AssetSource) -> Result<Box<dyn Any>, LoadAssetError> {
            let path = path.append(".toml");

            let mut asset_file = source.open_asset_file(&path).await?;
            let asset_file = asset_file.read_fully().await;
            let asset_file = String::from_utf8(asset_file).map_err(LoadAssetError::other)?;

            let bind_group: BindGroupLayoutAsset = toml::from_str(&asset_file).map_err(LoadAssetError::other)?;
            Ok(Box::new(bind_group))
        }
    }
}
