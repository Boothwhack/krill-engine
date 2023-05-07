use async_trait::async_trait;
use utils::{HList, hlist};
use utils::hlist::{Concat, IntoShape};
use crate::process::{ProcessBuilder};
use crate::surface::SurfaceResource;
use crate::wgpu_render::{setup_wgpu_render_resource, WGPURenderResource};
use crate::winit_surface::{setup_winit_resource, WinitSurface};

pub trait Platform {}

#[async_trait(? Send)]
pub trait PlatformWithDefaultSetup {
    type SetupInput: 'static;
    type SetupOutput: 'static;

    async fn setup(&mut self, input: Self::SetupInput) -> Self::SetupOutput;
}

pub async fn detect_platform() -> DefaultPlatform {
    DefaultPlatform {}
}

pub struct DefaultPlatform {}

impl Platform for DefaultPlatform {}

#[async_trait(? Send)]
impl PlatformWithDefaultSetup for DefaultPlatform {
    type SetupInput = ();
    type SetupOutput = HList!(
        SurfaceResource<WinitSurface>,
        WGPURenderResource,
    );

    async fn setup(&mut self, _input: Self::SetupInput) -> Self::SetupOutput {
        let winit_resource = setup_winit_resource();
        let wgpu_resource = setup_wgpu_render_resource(&winit_resource).await;

        hlist!(winit_resource, wgpu_resource)
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
