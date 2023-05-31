use crate::{DeviceContext, Frame, TextureFormat};

pub struct SurfaceContext {
    pub(crate) surface: wgpu::Surface,
    pub(crate) surface_config: Option<wgpu::SurfaceConfiguration>,
}

impl SurfaceContext {
    pub fn request_frame(&self) -> Frame {
        Frame {
            surface_texture: self.surface.get_current_texture()
                .expect("current surface texture for frame"),
        }
    }

    pub fn configure(&mut self, device: &DeviceContext, width: u32, height: u32) {
        let mut surface_config = self.surface.get_default_config(&device.adapter, width, height).expect("default surface configuration");
        let capabilities = self.surface.get_capabilities(&device.adapter);
        log::info!("Default surface configuration: {:?}", surface_config);
        log::info!("Surface capabilities: {:?}", capabilities);

        // prefer non-srgb for now while we don't support textures
        surface_config.format = match surface_config.format {
            TextureFormat::Rgba8UnormSrgb if capabilities.formats.contains(&TextureFormat::Rgba8Unorm) => TextureFormat::Rgba8Unorm,
            TextureFormat::Bgra8UnormSrgb if capabilities.formats.contains(&TextureFormat::Bgra8Unorm) => TextureFormat::Bgra8Unorm,
            _ => surface_config.format,
        };

        log::info!("Configuring surface with config: {:?}", surface_config);

        self.surface.configure(&device.device, &surface_config);
        self.surface_config = Some(surface_config);
    }

    pub fn present_frame(&self, frame: Frame) {
        frame.surface_texture.present();
    }

    pub fn format(&self) -> Option<TextureFormat> {
        self.surface_config.as_ref().map(|config| config.format)
    }

    pub fn size(&self) -> Option<(u32, u32)> {
        self.surface_config.as_ref().map(|config| (config.width, config.height))
    }
}
