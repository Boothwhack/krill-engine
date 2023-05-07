use std::future::{Future};
use async_trait::async_trait;
use log::debug;
use utils::{HList, hlist};
use utils::hlist::{Concat, IntoShape};
use crate::asset_resource::AssetSourceResource;
use crate::process::{ProcessBuilder};
use crate::surface::SurfaceResource;
use crate::wgpu_render::{setup_wgpu_render_resource, WGPUCompatible, WGPURenderResource};
use crate::winit_surface::{setup_winit_resource, WinitSurface};

#[cfg(target_family = "wasm")]
pub mod web {
    use web_sys::{HtmlCanvasElement};

    pub enum Placement {
        /// The canvas element will be placed into the DOM in the `<body>` tag.
        Default(HtmlCanvasElement),
        /// The canvas element has either been placed into the DOM by the application or does not
        /// need to be placed.
        DontPlace,
    }
}

pub trait Platform {
    fn spawn_local<F, Fut>(self, f: F)
        where Self: Sized,
              Fut: 'static + Future<Output=()>,
              F: FnOnce(Self) -> Fut;
}

#[async_trait(? Send)]
pub trait PlatformWithDefaultSetup {
    type SetupInput: 'static;
    type SetupOutput: 'static;

    async fn setup(&mut self, input: Self::SetupInput) -> Self::SetupOutput;
}

pub fn detect_platform() -> DefaultPlatform {
    DefaultPlatform {
        #[cfg(target_family = "wasm")]
        handle_canvas: None,
    }
}

pub struct DefaultPlatform {
    #[cfg(target_family = "wasm")]
    handle_canvas: Option<fn(web_sys::HtmlCanvasElement) -> web::Placement>,
}

impl Platform for DefaultPlatform {
    #[cfg(target_family = "wasm")]
    fn spawn_local<F, Fut>(self, f: F)
        where Self: Sized,
              Fut: 'static + Future<Output=()>,
              F: FnOnce(Self) -> Fut {
        wasm_bindgen_futures::spawn_local(f(self));
    }

    #[cfg(not(target_family = "wasm"))]
    fn spawn_local<F, Fut>(self, f: F)
        where Self: Sized,
              Fut: 'static + Future<Output=()>,
              F: FnOnce(Self) -> Fut {
        use tokio::runtime::Builder;

        let runtime = Builder::new_current_thread().build().unwrap();
        runtime.block_on(f(self));
    }
}

#[cfg(target_family = "wasm")]
impl DefaultPlatform {
    pub fn set_canvas_handler(&mut self, handler: fn(web_sys::HtmlCanvasElement) -> web::Placement) {
        self.handle_canvas = Some(handler);
    }
}

#[cfg(not(target_family = "wasm"))]
type DefaultPlatformAssetSource = assets::source::desktop_fs::DirectoryAssetSource;
#[cfg(target_family = "wasm")]
type DefaultPlatformAssetSource = assets::source::web_request::WebRequestAssetSource;

#[cfg(not(target_family = "wasm"))]
fn new_default_platform_asset_source() -> DefaultPlatformAssetSource {
    use assets::source::desktop_fs::DirectoryAssetSource;

    DirectoryAssetSource::new("assets")
}

#[cfg(target_family = "wasm")]
fn new_default_platform_asset_source() -> DefaultPlatformAssetSource {
    use assets::source::web_request::WebRequestAssetSource;

    let base_url = web_sys::window().unwrap()
        .location()
        .href().unwrap();
    let base_url = web_sys::Url::new_with_base("assets/", &base_url)
        .unwrap()
        .href();

    WebRequestAssetSource::new(base_url).unwrap()
}

#[async_trait(? Send)]
impl PlatformWithDefaultSetup for DefaultPlatform {
    type SetupInput = ();
    type SetupOutput = HList!(
        SurfaceResource<WinitSurface>,
        WGPURenderResource,
        AssetSourceResource<DefaultPlatformAssetSource>,
    );

    async fn setup(&mut self, _input: Self::SetupInput) -> Self::SetupOutput {
        #[cfg(target_family = "wasm")]
        console_error_panic_hook::set_once();

        let winit_resource = setup_winit_resource();

        #[cfg(target_family = "wasm")] {
            use winit::platform::web::WindowExtWebSys;

            let canvas = winit_resource.raw_window().canvas();
            debug!(target: "platform", "Handling Window canvas element.");
            match self.handle_canvas.unwrap_or(web::Placement::Default)(canvas) {
                web::Placement::Default(canvas) => {
                    debug!(target: "platform", "Placing canvas element in body.");
                    web_sys::window().unwrap()
                        .document().unwrap()
                        .body().unwrap()
                        .append_child(&canvas).unwrap();
                }
                web::Placement::DontPlace => {}
            }
        }

        let wgpu_resource = setup_wgpu_render_resource(&winit_resource).await;
        let asset_source_resource = AssetSourceResource::new(new_default_platform_asset_source());

        hlist!(winit_resource, wgpu_resource, asset_source_resource)
    }
}

#[async_trait(? Send)]
pub trait SetupPlatformDefaultsExt<R, P, I>
    where P: PlatformWithDefaultSetup,
          R: 'static + IntoShape<P::SetupInput, I>,
          R::Remainder: Concat {
    async fn setup_platform_defaults(self, platform: &mut P) -> ProcessBuilder<<R::Remainder as Concat>::Concatenated<P::SetupOutput>>;
}

#[async_trait(? Send)]
impl<R, P, I> SetupPlatformDefaultsExt<R, P, I> for ProcessBuilder<R>
    where P: PlatformWithDefaultSetup,
          R: 'static + IntoShape<P::SetupInput, I>,
          R::Remainder: Concat {
    async fn setup_platform_defaults(self, platform: &mut P) -> ProcessBuilder<<R::Remainder as Concat>::Concatenated<P::SetupOutput>> {
        self.setup_async(|input| platform.setup(input)).await
    }
}
