use crate::process::{ProcessBuilder};
use crate::surface::SurfaceResource;
use async_trait::async_trait;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use render::{DeviceContext, SurfaceContext, WGPUContext};
use utils::hlist::{Concat, Has, IntoShape};
use utils::{hlist, HList};

pub struct WGPURenderResource {
    wgpu_context: WGPUContext,
    surface_context: SurfaceContext,
    device_context: DeviceContext,
}

impl WGPURenderResource {
    pub fn surface(&self) -> &SurfaceContext {
        &self.surface_context
    }

    pub fn device(&self) -> &DeviceContext {
        &self.device_context
    }

    pub fn device_mut(&mut self) -> &mut DeviceContext {
        &mut self.device_context
    }

    pub fn get_mut(&mut self) -> (&mut SurfaceContext, &mut DeviceContext) {
        (&mut self.surface_context, &mut self.device_context)
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
        surface_context,
        device_context
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
