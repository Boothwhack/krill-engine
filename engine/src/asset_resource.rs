use std::ops::{Deref, DerefMut};
use assets::source::AssetSource;

pub struct AssetSourceResource<A: AssetSource> {
    asset_source: A,
}

impl<A: AssetSource> AssetSourceResource<A> {
    pub fn new(asset_source: A) -> Self {
        AssetSourceResource { asset_source }
    }
}

impl<A: AssetSource> DerefMut for AssetSourceResource<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.asset_source
    }
}

impl<A: AssetSource> Deref for AssetSourceResource<A> {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        &self.asset_source
    }
}

#[cfg(not(target_family = "wasm"))]
pub mod desktop {
    use std::path::PathBuf;
    use assets::source::desktop_fs::DirectoryAssetSource;
    use utils::{HList, hlist};
    use utils::hlist::{Concat, IntoShape};
    use crate::asset_resource::AssetSourceResource;
    use crate::process::{ProcessBuilder};

    pub trait DirectoryAssetSourceExt<R, I, P: Into<PathBuf>> {
        type Output;

        fn setup_directory_asset_source(self, path: P) -> Self::Output;
    }

    impl<R, I, P> DirectoryAssetSourceExt<R, I, P> for ProcessBuilder<R>
        where P: Into<PathBuf>,
              R: 'static + IntoShape<HList!(), I>,
              R::Remainder: Concat {
        type Output = ProcessBuilder<<R::Remainder as Concat>::Concatenated<HList!(AssetSourceResource<DirectoryAssetSource>)>>;//ProcessBuilderWith<R, I, DirectoryAssetSourceSetupStep<P>>;

        fn setup_directory_asset_source(self, path: P) -> Self::Output {
            self.setup(move |_| {
                hlist!(AssetSourceResource::new(DirectoryAssetSource::new(path)))
            })
        }
    }
}

#[cfg(target_family = "wasm")]
pub mod web {
    use assets::source::web_request::{IntoUrl, WebRequestAssetSource};
    use utils::{HList, hlist};
    use utils::hlist::{Concat, IntoShape};
    use crate::asset_resource::AssetSourceResource;
    use crate::process::{ProcessBuilder};

    pub trait WebRequestAssetSourceExt<R, I, U: IntoUrl> {
        type Output;

        fn setup_web_request_asset_source(self, url: U) -> Self::Output;
    }

    impl<R, I, U> WebRequestAssetSourceExt<R, I, U> for ProcessBuilder<R>
        where U: IntoUrl,
              R: IntoShape<HList!(), I>,
              R::Remainder: Concat {
        type Output = ProcessBuilder<<R::Remainder as Concat>::Concatenated<HList!(AssetSourceResource<WebRequestAssetSource>)>>;//ProcessBuilderWith<R, I, DirectoryAssetSourceSetupStep<P>>;

        fn setup_web_request_asset_source(self, url: U) -> Self::Output {
            self.setup(move |_| {
                hlist!(AssetSourceResource::new(WebRequestAssetSource::new(url).unwrap()))
            })
        }
    }
}
