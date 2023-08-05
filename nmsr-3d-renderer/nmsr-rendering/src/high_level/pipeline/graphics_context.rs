use std::{borrow::Cow, mem};

use glam::Vec3;
pub use wgpu::{
    Adapter, Backends, Device, Instance, Queue, Surface, SurfaceConfiguration, TextureFormat,
};
use wgpu::{
    BindGroupDescriptor, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendState,
    BufferAddress, BufferBindingType, ColorTargetState, ColorWrites, CompareFunction,
    DepthStencilState, FragmentState, PipelineLayoutDescriptor, RenderPipelineDescriptor,
    ShaderModuleDescriptor, ShaderStages, VertexBufferLayout, BindGroupLayout, RenderPipeline, TextureViewDimension, TextureSampleType,
};

use crate::{
    errors::{NMSRRenderingError, Result},
    low_level::primitives::vertex::Vertex,
};

use super::scene::Size;

#[derive(Debug)]
pub struct GraphicsContext {
    pub instance: Instance,
    pub device: Device,
    pub queue: Queue,
    pub surface: Option<Surface>,
    pub surface_config: Option<Result<SurfaceConfiguration>>,
    pub surface_view_format: Option<TextureFormat>,
    pub adapter: Adapter,

    pub pipeline: RenderPipeline,
    pub transform_bind_group_layout: BindGroupLayout,
    pub skin_bind_group_layout: BindGroupLayout,
}

pub type ServiceProvider<'a> = dyn FnOnce(&Instance) -> Option<Surface> + 'a;

pub struct GraphicsContextDescriptor<'a> {
    pub backends: Option<Backends>,
    pub surface_provider: Box<ServiceProvider<'a>>,
    pub default_size: (u32, u32),
    pub texture_format: Option<TextureFormat>,
}

impl GraphicsContext {
    pub const DEFAULT_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

    pub async fn new(descriptor: GraphicsContextDescriptor<'_>) -> Result<Self> {
        let texture_format = descriptor.texture_format.unwrap_or(Self::DEFAULT_TEXTURE_FORMAT);
        
        let backends = wgpu::util::backend_bits_from_env()
            .or(descriptor.backends)
            .ok_or(NMSRRenderingError::NoBackendFound)?;

        let dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default();

        let instance = Instance::new(wgpu::InstanceDescriptor {
            backends,
            dx12_shader_compiler,
        });

        let surface = (descriptor.surface_provider)(&instance);

        let adapter =
            wgpu::util::initialize_adapter_from_env_or_default(&instance, surface.as_ref())
                .await
                .ok_or(NMSRRenderingError::NoAdapterFound)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await?;

        let (default_width, default_height) = descriptor.default_size;

        let mut surface_config = surface.as_ref().map(|surface| {
            surface
                .get_default_config(&adapter, default_width, default_height)
                .ok_or(NMSRRenderingError::SurfaceNotSupported)
        });

        let surface_view_format = surface_config
            .as_ref()
            .and_then(|s| s.as_ref().ok().map(|s| s.format));

        if let Some(surface) = &surface {
            if let Some(surface_view_format) = surface_view_format {
                if let Some(Ok(surface_config)) = surface_config.as_mut() {
                    surface_config.view_formats.push(surface_view_format);
                    surface.configure(&device, surface_config);
                }
            }
        }

        let adapter =
            wgpu::util::initialize_adapter_from_env_or_default(&instance, surface.as_ref())
                .await
                .ok_or(NMSRRenderingError::WgpuAdapterRequestError)?;

        // Create a bind group layout for storing the transformation matrix in a uniform
        let transform_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(64),
                    },
                    count: None,
                }],
                label: Some("Transform Bind Group Layout"),
            });
            
            
        let skin_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Skin Texture Bind Group"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    multisampled: false,
                    view_dimension: TextureViewDimension::D2,
                    sample_type: TextureSampleType::Float { filterable: true },
                },
                count: None,
            }],
        });
        
        let skin_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Skin Texture Bind Group"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    multisampled: false,
                    view_dimension: TextureViewDimension::D2,
                    sample_type: TextureSampleType::Float { filterable: true },
                },
                count: None,
            }],
        });

        // Create the pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Scene Pipeline Layout"),
            bind_group_layouts: &[&transform_bind_group_layout, &skin_bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let vertex_buffer_layout = VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: mem::size_of::<Vec3>() as BufferAddress,
                    shader_location: 1,
                },
            ],
        };

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[vertex_buffer_layout],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                front_face: wgpu::FrontFace::Cw,
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: texture_format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        Ok(GraphicsContext {
            instance,
            device,
            queue,
            surface,
            surface_config,
            surface_view_format,
            adapter,
            pipeline,
            transform_bind_group_layout,
            skin_bind_group_layout
        })
    }

    pub fn set_surface_size(&mut self, size: Size) {
        if let Some(Ok(config)) = &mut self.surface_config {
            config.width = size.width;
            config.height = size.height;
            
            if let Some(surface) = &self.surface {
                surface.configure(&self.device, config);
            }
        }
    }
}
