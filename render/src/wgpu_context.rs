use crate::{DeviceContext, SurfaceContext};

pub struct WGPUContext {
    instance: wgpu::Instance,
}

impl WGPUContext {
    // enumerate_adapters is not available in wasm environments
    #[cfg(not(target_family = "wasm"))]
    fn log_adapters(instance: &wgpu::Instance) {
        log::info!("Adapters:");
        for adapter in instance.enumerate_adapters(wgpu::Backends::all()) {
            log::info!("  {:?}", adapter.get_info());
        }
    }

    pub async fn new() -> Option<Self> {
        let instance = wgpu::Instance::default();

        #[cfg(not(target_family = "wasm"))]
        WGPUContext::log_adapters(&instance);

        log::info!("Got WGPU instance.");

        Some(WGPUContext { instance })
    }

    pub async fn request_device(&self, surface: &SurfaceContext) -> Result<DeviceContext, wgpu::RequestDeviceError> {
        let adapter = self.instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface.surface),
            ..Default::default()
        }).await.expect("viable adapter");
        log::info!("Got adapter: {:?}", adapter.get_info());
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                limits: wgpu::Limits::downlevel_webgl2_defaults(),

                ..Default::default()
            },
            None,
        ).await?;
        Ok(DeviceContext::new(adapter, device, queue))
    }

    pub fn create_surface<W>(&self, window: &W) -> SurfaceContext
        where W: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle {
        log::info!("Creating surface...");
        let surface = unsafe { self.instance.create_surface(window) }.expect("surface");

        SurfaceContext {
            surface,
            surface_config: None,
        }
    }
}
