use assets::source::AssetSource;
use crate::process::ProcessBuilder;
use crate::resource::ResourceList;

pub struct AssetSourceResource<A: AssetSource> {
    asset_source: A,
}

impl<A: AssetSource> AssetSourceResource<A> {
    pub fn get(&self) -> &A {
        &self.asset_source
    }
}

#[cfg(target_family = "wasm")]
pub mod web {
    use assets::source::web_request::{IntoUrl, WebRequestAssetSource};
    use crate::asset_resource::AssetSourceResource;
    use crate::process::ProcessBuilder;
    use crate::resource::ResourceList;

    pub trait WithWebAssetSourceExt<R: ResourceList> {
        fn with_web_asset_source<U: IntoUrl>(self, base_url: U) -> ProcessBuilder<R::WithResource<AssetSourceResource<WebRequestAssetSource>>>;
    }

    impl<R: ResourceList> WithWebAssetSourceExt<R> for ProcessBuilder<R> {
        fn with_web_asset_source<U: IntoUrl>(self, base_url: U) -> ProcessBuilder<R::WithResource<AssetSourceResource<WebRequestAssetSource>>> {
            self.setup(|resources| {
                let asset_source = WebRequestAssetSource::new(base_url)
                    .expect("invalid asset source url");
                resources.with_resource(AssetSourceResource { asset_source })
            })
        }
    }
}

/*#[cfg(target_family = "wasm")]
pub type PlatformAssetSource = WebRequestAssetSource;*/

#[cfg(not(target_family = "wasm"))]
pub mod desktop {
    use std::path::PathBuf;
    use assets::source::desktop_fs::DirectoryAssetSource;
    use crate::asset_resource::AssetSourceResource;
    use crate::process::ProcessBuilder;
    use crate::resource::ResourceList;

    pub trait WithDirectoryAssetSourceExt<R: ResourceList> {
        fn with_directory_asset_source<P: Into<PathBuf>>(self, path: P) -> ProcessBuilder<R::WithResource<AssetSourceResource<DirectoryAssetSource>>>;
    }

    impl<R: ResourceList> WithDirectoryAssetSourceExt<R> for ProcessBuilder<R> {
        fn with_directory_asset_source<P: Into<PathBuf>>(self, path: P) -> ProcessBuilder<R::WithResource<AssetSourceResource<DirectoryAssetSource>>> {
            self.setup(|resources| {
                let asset_source = DirectoryAssetSource::new(path);
                resources.with_resource(AssetSourceResource { asset_source })
            })
        }
    }
}

/*#[cfg(not(target_family = "wasm"))]
pub type PlatformAssetSource = DirectoryAssetSource;*/

/*pub trait WithAssetResourceExt<R: ResourceList, A: AssetSource> {
    fn with_asset_source(self) -> ProcessBuilder<R::WithResource<A>>;
}

#[cfg(target_family = "wasm")]
impl<R: ResourceList> WithAssetResourceExt<R, assets::source::web_request::WebRequestAssetSource> for ProcessBuilder<R> {
    fn with_asset_source(self) -> ProcessBuilder<R::WithResource<assets::source::web_request::WebRequestAssetSource>> {
        use assets::source::web_request::WebRequestAssetSource;

        self.setup(|resources| resources.with_resource(WebRequestAssetSource::default()))
    }
}

#[cfg(not(target_family = "wasm"))]
impl<R: ResourceList> WithAssetResourceExt<R, assets::source::desktop_fs::DirectoryAssetSource> for ProcessBuilder<R> {
    fn with_asset_source(self) -> ProcessBuilder<R::WithResource<assets::source::desktop_fs::DirectoryAssetSource>> {
        use assets::source::desktop_fs::DirectoryAssetSource;

        self.setup(|resources| resources.with_resource(DirectoryAssetSource::new("assets")))
    }
}*/
