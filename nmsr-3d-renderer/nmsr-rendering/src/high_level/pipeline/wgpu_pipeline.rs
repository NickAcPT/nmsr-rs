pub use wgpu::{
    Adapter, Backends, Device, Instance, Queue, Surface, SurfaceConfiguration, TextureFormat,
};

use crate::errors::{NMSRRenderingError, Result};

#[derive(Debug)]
pub struct GraphicsContext {
    pub instance: Instance,
    pub device: Device,
    pub queue: Queue,
    pub surface: Option<Surface>,
    pub surface_config: Option<Result<SurfaceConfiguration>>,
    pub surface_view_format: Option<TextureFormat>,
    pub adapter: Adapter,
}

pub type ServiceProvider<'a> = dyn FnOnce(&Instance) -> Option<Surface> + 'a;

pub struct GraphicsContextDescriptor<'a> {
    pub backends: Option<Backends>,
    pub surface_provider: Box<ServiceProvider<'a>>,
    pub default_size: (u32, u32),
}

impl GraphicsContext {
    pub async fn new(descriptor: GraphicsContextDescriptor<'_>) -> Result<Self> {
        let backends = wgpu::util::backend_bits_from_env().or(descriptor.backends).ok_or(NMSRRenderingError::NoBackendFound)?;

        let dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default();

        let instance = Instance::new(wgpu::InstanceDescriptor {
            backends,
            dx12_shader_compiler,
        });

        let surface = (descriptor.surface_provider)(&instance);

        let adapter = wgpu::util::initialize_adapter_from_env_or_default(&instance, surface.as_ref()).await.ok_or(NMSRRenderingError::NoAdapterFound)?;

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            None,
        ).await?;

        let (default_width, default_height) = descriptor.default_size;

        let mut surface_config = surface.as_ref().map(|surface| {
            surface.get_default_config(&adapter, default_width, default_height).ok_or(NMSRRenderingError::SurfaceNotSupported)
        });

        let surface_view_format = surface_config.as_ref().and_then(|s| s.as_ref().ok().map(|s| s.format));

        if let Some(surface) = &surface {
            if let Some(surface_view_format) = surface_view_format {
                if let Some(Ok(surface_config)) = surface_config.as_mut() {
                    surface_config.view_formats.push(surface_view_format);
                    surface.configure(&device, surface_config);
                }
            }
        }

        let adapter = wgpu::util::initialize_adapter_from_env_or_default(&instance, surface.as_ref()).await.ok_or(NMSRRenderingError::WgpuAdapterRequestError)?;

        Ok(GraphicsContext {
            instance,
            device,
            queue,
            surface,
            surface_config,
            surface_view_format,
            adapter,
        })
    }
}
