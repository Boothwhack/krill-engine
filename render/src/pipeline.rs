pub mod serial {
    use async_trait::async_trait;
    use assets::{AssetPipeline, LoadAssetError};
    use assets::path::AssetPath;
    use assets::source::AssetSource;
    use std::any::Any;
    use std::collections::HashMap;
    use std::str::FromStr;
    use serde::{Deserialize, Deserializer};
    use thiserror::Error;

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "kebab-case")]
    pub struct PipelineDefinition {
        pub shader_modules: Vec<ShaderModuleDefinition>,
        pub bind_groups: Vec<BindGroupLayoutDefinition>,
        pub vertex_shader: VertexShaderDefinition,
        pub fragment_shader: Option<FragmentShaderDefinition>,
        #[serde(default)]
        pub primitive: PrimitiveDefinition,
    }

    #[derive(Default, Deserialize, Debug)]
    #[serde(rename_all = "kebab-case")]
    pub struct PrimitiveDefinition {
        #[serde(default)]
        pub topology: Topology,
        #[serde(default)]
        pub winding: FaceWinding,
        #[serde(default)]
        pub cull: Option<Cull>,
    }

    #[derive(Default, Deserialize, Debug)]
    #[serde(rename_all = "kebab-case")]
    pub enum Topology {
        Points,
        Lines,
        LineStrip,
        #[default]
        Triangles,
        TriangleStrip,
    }

    #[derive(Default, Deserialize, Debug)]
    #[serde(rename_all = "kebab-case")]
    pub enum FaceWinding {
        #[default]
        CounterClockwise,
        Clockwise,
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "kebab-case")]
    pub enum Cull {
        Front,
        Back,
    }

    #[derive(Deserialize, Debug)]
    pub struct BindGroupLayoutDefinition {
        pub layout: String,
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "kebab-case")]
    pub struct ShaderModuleDefinition {
        pub name: String,
        pub path: String,
    }

    #[derive(Default, Deserialize, Debug)]
    #[serde(rename_all = "kebab-case")]
    pub enum VertexShaderStepMode {
        #[default]
        Vertex,
        Instance,
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "kebab-case")]
    pub struct VertexShaderDefinition {
        pub shader_module: String,
        pub entrypoint: String,
        pub buffers: Vec<VertexBufferDefinition>,
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "kebab-case")]
    pub struct VertexBufferDefinition {
        pub step_mode: VertexShaderStepMode,
        pub attributes: Vec<VertexBufferAttributeDefinition>,
    }

    impl VertexBufferDefinition {
        pub fn stride(&self) -> u64 {
            self.attributes
                .iter()
                .map(|attr| attr.format.size())
                .sum()
        }
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "kebab-case")]
    pub struct VertexBufferAttributeDefinition {
        pub format: VertexFormatDefinition,
    }

    #[derive(Debug)]
    pub enum VertexFormatDefinition {
        Float32(u32),
        Float64(u32),
    }

    impl VertexFormatDefinition {
        pub fn size(&self) -> u64 {
            match self {
                VertexFormatDefinition::Float32(count) => 4 * count,
                VertexFormatDefinition::Float64(count) => 8 * count,
            }.into()
        }
    }

    impl<'de> Deserialize<'de> for VertexFormatDefinition {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
            let str = String::deserialize(deserializer)?;
            VertexFormatDefinition::from_str(&str)
                .map_err(serde::de::Error::custom)
        }
    }

    #[derive(Debug, Error)]
    pub enum InvalidVertexFormatString {
        #[error("invalid element count")]
        InvalidCount,
        #[error("unknown format")]
        UnknownFormat,
        #[error("element count out of range")]
        OutOfRange,
    }

    impl FromStr for VertexFormatDefinition {
        type Err = InvalidVertexFormatString;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            use VertexFormatDefinition::*;

            let parts: Vec<_> = s.splitn(2, "x").collect();

            let count = if parts.len() == 2 {
                u32::from_str(parts[1])
                    .map_err(|_| InvalidVertexFormatString::InvalidCount)?
            } else {
                1
            };
            let (variant, count_range): (fn(u32) -> VertexFormatDefinition, _) = match parts[0] {
                "f32" => (Float32, 1..=4),
                "f64" => (Float64, 1..=4),
                _ => return Err(InvalidVertexFormatString::UnknownFormat),
            };

            if count_range.contains(&count) {
                Ok(variant(count))
            } else {
                Err(InvalidVertexFormatString::OutOfRange)
            }
        }
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "kebab-case")]
    pub struct FragmentShaderDefinition {
        pub shader_module: String,
        pub entrypoint: String,
        pub targets: Vec<FragmentShaderTargetDefinition>,
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "kebab-case")]
    pub struct FragmentShaderTargetDefinition {
        pub format: TargetFormat,
    }

    #[derive(Deserialize, Debug)]
    pub enum TargetFormat {
        #[serde(rename = "bgra8-unorm-srgb")]
        BGRA8UnormSRGB,
        #[serde(rename = "rgba8-unorm-srgb")]
        RGBA8UnormSRGB,
        /** Use same format as the surfaces texture. */
        #[serde(rename = "surface")]
        Surface,
    }

    pub struct RenderPipelineAssetPipeline;

    pub struct RenderPipelineAsset {
        pub shader_modules: HashMap<String, String>,
        pub definition: PipelineDefinition,
    }

    #[async_trait(? Send)]
    impl AssetPipeline for RenderPipelineAssetPipeline {
        async fn load_asset(&self, path: AssetPath, source: &dyn AssetSource) -> Result<Box<dyn Any>, LoadAssetError> {
            let path = path.append(".toml");

            // load the main asset definition file in TOML format
            let mut pipeline_file = source.open_asset_file(&path).await?;
            let pipeline_file = pipeline_file.read_fully().await;
            let pipeline_file = String::from_utf8(pipeline_file).map_err(LoadAssetError::other)?;

            let pipeline_definition: PipelineDefinition = toml::from_str(&pipeline_file)
                .map_err(LoadAssetError::other)?;

            // discover all necessary shader modules and load their sources
            let mut shader_modules: HashMap<String, String> = HashMap::new();
            for module in pipeline_definition.shader_modules.iter() {
                if shader_modules.contains_key(&module.name) {
                    continue;
                }

                let module_path = AssetPath::new(&module.path).map_err(LoadAssetError::InvalidPath)?;
                let module_path = path.resolve(module_path.clone()).ok_or_else(|| LoadAssetError::NotFound(module_path))?;

                let mut module_file = source.open_asset_file(&module_path).await?;
                let module_file = module_file.read_fully().await;
                let module_file = String::from_utf8(module_file).map_err(LoadAssetError::other)?;

                shader_modules.insert(module.name.clone(), module_file);
            }

            Ok(Box::new(RenderPipelineAsset {
                shader_modules,
                definition: pipeline_definition,
            }))
        }
    }
}
