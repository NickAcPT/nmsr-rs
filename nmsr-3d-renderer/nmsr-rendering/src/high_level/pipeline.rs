use wgpu::{Device, Instance as WgpuInstance, Surface};

use crate::high_level::errors::{NMSRRenderingError, Result};

pub struct NmsrPipeline {
    wgpu_instance: WgpuInstance,
    wgpu_device: Device,
    wgpu_surface: Surface
}

pub struct NmsrPipelineDescriptor<'a> {
    pub backends: Option<wgpu::Backends>,
    pub surface_provider: Box<dyn FnOnce(&WgpuInstance) -> Surface + 'a>,
}

impl From<NmsrPipeline> for (WgpuInstance, Device, Surface) {
    fn from(value: NmsrPipeline) -> Self {
        (value.wgpu_instance, value.wgpu_device, value.wgpu_surface)
    }
}

#[allow(unreachable_code)]
impl NmsrPipeline {
    pub async fn new(descriptor: NmsrPipelineDescriptor<'_>) -> Result<Self> {
        let backends = wgpu::util::backend_bits_from_env().or(descriptor.backends).ok_or(NMSRRenderingError::NoBackendFound)?;
        let dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default();

        // Important:
        let wgpu_instance = WgpuInstance::new(wgpu::InstanceDescriptor {
            backends,
            dx12_shader_compiler,
        });

        let surface: Surface = (descriptor.surface_provider)(&wgpu_instance);

        let adapter = wgpu::util::initialize_adapter_from_env_or_default(&wgpu_instance, Some(&surface))
            .await
            .ok_or(NMSRRenderingError::NoAdapterFound)?;

        let (wgpu_device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            ).await?;

        let mut config = surface
            .get_default_config(&adapter, 1, 1)
            .ok_or(NMSRRenderingError::NoAdapterFound)?;

        let surface_view_format = config.format;
        config.view_formats.push(surface_view_format);
        surface.configure(&wgpu_device, &config);

        Ok(NmsrPipeline {
            wgpu_instance,
            wgpu_device,
            wgpu_surface: surface
        })
    }
}