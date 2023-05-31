use std::ops::{Deref, DerefMut};
use crate::process::ProcessBuilder;
use crate::surface::SurfaceResource;
use async_trait::async_trait;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use render::WGPUContext;
use render::RenderApi;
use utils::hlist::{Concat, Has, IntoShape};
use utils::{hlist, HList};

pub struct WGPURenderResource {
    wgpu_context: WGPUContext,
    render_api: RenderApi,
}

impl DerefMut for WGPURenderResource {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.render_mut()
    }
}

impl Deref for WGPURenderResource {
    type Target = RenderApi;

    fn deref(&self) -> &Self::Target {
        self.render()
    }
}

impl WGPURenderResource {
    pub fn render(&self) -> &RenderApi {
        &self.render_api
    }

    pub fn render_mut(&mut self) -> &mut RenderApi {
        &mut self.render_api
    }
}

pub trait WGPUCompatible {
    type RawWindow: HasRawWindowHandle + HasRawDisplayHandle;

    fn raw_window(&self) -> &Self::RawWindow;

    fn size(&self) -> (u32, u32);
}

pub async fn setup_wgpu_render_resource<S>(surface: &SurfaceResource<S>) -> WGPURenderResource
    where S: WGPUCompatible {
    let wgpu_context = WGPUContext::new().await.unwrap();
    let mut surface_context = wgpu_context.create_surface(surface.raw_window());
    let device_context = wgpu_context.request_device(&surface_context).await.unwrap();

    let (width, height) = surface.size();
    surface_context.configure(&device_context, width, height);

    WGPURenderResource {
        wgpu_context,
        render_api: RenderApi::new(device_context, surface_context),
    }
}

#[async_trait(? Send)]
pub trait WGPURenderSetupExt<S: WGPUCompatible, I> {
    type Output;

    async fn setup_wgpu_render(self) -> Self::Output;
}

#[async_trait(? Send)]
impl<R, I, S> WGPURenderSetupExt<S, I> for ProcessBuilder<R>
    where
        S: 'static + WGPUCompatible,
        R: 'static + IntoShape<HList!(SurfaceResource<S>), I>,
        R::Remainder: Concat,
{
    type Output = ProcessBuilder<<R::Remainder as Concat>::Concatenated<HList!(WGPURenderResource, SurfaceResource<S>)>>;

    async fn setup_wgpu_render(self) -> Self::Output {
        self.setup_async(|resources| async {
            let (surface, _): (SurfaceResource<S>, _) = resources.pick();

            hlist!(
                setup_wgpu_render_resource(&surface).await,
                surface
            )
        }).await
    }
}
