use wgpu::{Adapter as WgpuAdapter, Instance as WgpuInstance, Device as WgpuDevice, Queue as WgpuQueue, Surface as WgpuSurface, RequestAdapterOptions, SurfaceConfiguration as WgpuSurfaceConfiguration};

use crate::high_level::errors::{NMSRRenderingError, Result};

pub struct NmsrPipeline {
    pub wgpu_instance: WgpuInstance,
    pub wgpu_device: WgpuDevice,
    pub wgpu_queue: WgpuQueue,
    pub wgpu_surface: WgpuSurface,
    pub wgpu_surface_config: WgpuSurfaceConfiguration,
    pub wgpu_adapter: WgpuAdapter,
}

pub struct NmsrPipelineDescriptor<'a> {
    pub backends: Option<wgpu::Backends>,
    pub surface_provider: Box<dyn FnOnce(&WgpuInstance) -> WgpuSurface + 'a>,
    pub default_size: (u32, u32),
}

#[allow(unreachable_code)]
impl NmsrPipeline {
    pub async fn new(descriptor: NmsrPipelineDescriptor<'_>) -> Result<Self> {
        let backends = wgpu::util::backend_bits_from_env().or(descriptor.backends).ok_or(NMSRRenderingError::NoBackendFound)?;
        let dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default();

        let instance = WgpuInstance::new(wgpu::InstanceDescriptor {
            backends,
            dx12_shader_compiler,
        });

        let surface = (descriptor.surface_provider)(&instance);

        let adapter = wgpu::util::initialize_adapter_from_env_or_default(&instance, Some(&surface))
            .await
            .ok_or(NMSRRenderingError::NoAdapterFound)?;

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            None,
        ).await?;

        let (default_width, default_height) = descriptor.default_size;

        let mut surface_config = surface
            .get_default_config(&adapter, default_width, default_height)
            .ok_or(NMSRRenderingError::SurfaceNotSupported)?;

        let surface_view_format = surface_config.format;
        surface_config.view_formats.push(surface_view_format);
        surface.configure(&device, &surface_config);

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .ok_or(NMSRRenderingError::WgpuAdapterRequestError)?;

        Ok(NmsrPipeline {
            wgpu_instance: instance,
            wgpu_device: device,
            wgpu_queue: queue,
            wgpu_surface: surface,
            wgpu_surface_config: surface_config,
            wgpu_adapter: adapter
        })
    }
}