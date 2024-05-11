use std::{borrow::Cow, env, mem, sync::Arc};

use deadpool::managed::{Object, Pool};
use smaa::SmaaMode;
use wgpu::{
    vertex_attr_array, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingType, BufferAddress, BufferBindingType, BufferSize, ColorTargetState, ColorWrites,
    CompareFunction, DepthStencilState, FragmentState, FrontFace, MultisampleState,
    PipelineLayoutDescriptor, PresentMode, PrimitiveState, RenderPipeline,
    RenderPipelineDescriptor, SamplerBindingType, ShaderModuleDescriptor, ShaderStages,
    TextureSampleType, TextureViewDimension, VertexBufferLayout, VertexState,
};
pub use wgpu::{
    Adapter, Backends, BlendState, Device, Features, Instance, Queue, ShaderSource, Surface,
    SurfaceConfiguration, TextureFormat, Limits,
};

use crate::{
    errors::{NMSRRenderingError, Result},
    low_level::primitives::vertex::Vertex,
};

use super::{
    pools::SceneContextPoolManager,
    scene::{Size, SunInformation},
};

#[derive(Debug)]
pub struct GraphicsContext<'a> {
    pub instance: Instance,
    pub device: Device,
    pub queue: Queue,
    pub surface: Option<Surface<'a>>,
    pub surface_config: Result<Option<SurfaceConfiguration>>,
    pub texture_format: TextureFormat,
    pub adapter: Adapter,

    pub pipeline: RenderPipeline,
    pub layouts: GraphicsContextLayouts,
    pub multisampling_strategy: MultiSamplingStrategy,
}

#[derive(Debug)]
pub enum MultiSamplingStrategy {
    MSAA(u32),
    SMAA(SmaaMode),
    SMAAWithMSAA((SmaaMode, u32)),
}

impl MultiSamplingStrategy {
    pub fn get_smaa_mode(&self) -> SmaaMode {
        match self {
            Self::SMAA(mode) | Self::SMAAWithMSAA((mode, _)) => *mode,
            _ => SmaaMode::Disabled,
        }
    }

    pub fn get_msaa_sample_count(&self) -> u32 {
        match self {
            Self::MSAA(count) | Self::SMAAWithMSAA((_, count)) => *count,
            _ => 1,
        }
    }
}

#[derive(Debug)]
pub struct GraphicsContextLayouts {
    pub transform_bind_group_layout: BindGroupLayout,
    pub skin_sampler_bind_group_layout: BindGroupLayout,
    pub pipeline_layout: wgpu::PipelineLayout,
    pub sun_bind_group_layout: BindGroupLayout,
}

#[derive(Debug)]
pub struct GraphicsContextPools<'a> {
    scene_context_pool: Pool<SceneContextPoolManager<'a>>,
}

impl<'a> GraphicsContextPools<'a> {
    pub fn new(context: Arc<GraphicsContext<'a>>) -> Result<Self> {
        let scene_context_pool = Pool::builder(SceneContextPoolManager::new(context)).build()?;

        Ok(Self { scene_context_pool })
    }

    pub async fn create_scene_context(&self) -> Result<Object<SceneContextPoolManager<'a>>> {
        Ok(self.scene_context_pool.get().await?)
    }
}

impl<'a> GraphicsContext<'a> {
    pub fn get_pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }
}

pub type ServiceProvider<'a> = dyn FnOnce(&Instance) -> Option<Surface<'a>> + 'a + Send;

pub struct GraphicsContextDescriptor<'a> {
    pub backends: Option<Backends>,
    pub surface_provider: Box<ServiceProvider<'a>>,
    pub default_size: (u32, u32),
    pub texture_format: Option<TextureFormat>,
    pub features: Features,
    pub limits: Option<Limits>,
    pub blend_state: Option<BlendState>,
    pub sample_count: Option<u32>,
    pub use_smaa: Option<bool>,
}

impl<'a> GraphicsContextDescriptor<'a> {
    pub(crate) fn get_multisampling_strategy(
        adapter: &Adapter,
        texture_format: &TextureFormat,
        use_smaa: Option<bool>,
        sample_count: Option<u32>,
    ) -> MultiSamplingStrategy {
        let wants_smaa = use_smaa.unwrap_or(env::var("NMSR_USE_SMAA").is_ok());

        let format = *texture_format;
        let sample_flags = adapter.get_texture_format_features(format).flags;

        let env_sample_count = env::var("NMSR_SAMPLE_COUNT")
            .ok()
            .and_then(|it| it.parse::<u32>().ok());

        let count = sample_count.or(env_sample_count).unwrap_or_else(|| {
            [16, 8, 4, 2, 1]
                .iter()
                .find(|&&sample_count| sample_flags.sample_count_supported(sample_count))
                .copied()
                .unwrap_or(1)
        });

        let mut strat = MultiSamplingStrategy::MSAA(count);

        if wants_smaa {
            strat = MultiSamplingStrategy::SMAAWithMSAA((
                SmaaMode::Smaa1X,
                strat.get_msaa_sample_count(),
            ));
        }

        strat
    }
}

impl<'a> GraphicsContext<'a> {
    pub const DEFAULT_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
    pub const DEPTH_TEXTURE_FORMAT: TextureFormat = TextureFormat::Depth32Float;

    pub async fn new(descriptor: GraphicsContextDescriptor<'a>) -> Result<Self> {
        Self::new_with_shader(
            descriptor,
            wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        )
        .await
    }

    #[inline]
    pub async fn new_with_shader(
        descriptor: GraphicsContextDescriptor<'a>,
        shader: ShaderSource<'_>,
    ) -> Result<Self> {
        let backends = wgpu::util::backend_bits_from_env()
            .or(descriptor.backends)
            .ok_or(NMSRRenderingError::NoBackendFound)?;

        let dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default();

        let instance = Instance::new(wgpu::InstanceDescriptor {
            backends,
            dx12_shader_compiler,
            ..Default::default()
        });

        let mut surface = (descriptor.surface_provider)(&instance);

        let adapter =
            wgpu::util::initialize_adapter_from_env_or_default(&instance, surface.as_ref())
                .await
                .ok_or(NMSRRenderingError::NoAdapterFound)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: descriptor.features,
                    required_limits: descriptor.limits.unwrap_or_else(|| wgpu::Limits::default())
                },
                None,
            )
            .await?;

        let (default_width, default_height) = descriptor.default_size;

        let mut surface_config = surface
            .as_mut()
            .map(|surface| {
                surface
                    .get_default_config(&adapter, default_width, default_height)
                    .ok_or(NMSRRenderingError::SurfaceNotSupported)
            })
            .transpose();

        if let Some(surface) = &surface {
            if let Ok(Some(surface_config)) = surface_config.as_mut() {
                surface_config.view_formats.push(surface_config.format);
                surface_config.present_mode = PresentMode::AutoVsync;
                surface.configure(&device, surface_config);
            }
        }

        let surface_view_format = {
            surface_config
                .as_ref()
                .map(|s| s.as_ref().map(|s| s.format))
        };

        let texture_format = surface_view_format
            .ok()
            .flatten()
            .or(descriptor.texture_format)
            .unwrap_or(Self::DEFAULT_TEXTURE_FORMAT);

        let adapter =
            wgpu::util::initialize_adapter_from_env_or_default(&instance, surface.as_ref())
                .await
                .ok_or(NMSRRenderingError::WgpuAdapterRequestError)?;

        // Create a bind group layout for storing the transformation matrix in a uniform
        let transform_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(64),
                    },
                    count: None,
                }],
                label: Some("Transform Bind Group Layout"),
            });

        let skin_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Texture Bind Group"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let sun_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Sun Bind Group"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(mem::size_of::<SunInformation>() as u64),
                },
                count: None,
            }],
        });

        // Create the pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Scene Pipeline Layout"),
            bind_group_layouts: &[
                &transform_bind_group_layout,
                &skin_bind_group_layout,
                &sun_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: shader,
        });

        let vertex_buffer_layout = VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x3],
        };

        let multisampling_strategy = GraphicsContextDescriptor::get_multisampling_strategy(
            &adapter,
            &texture_format,
            descriptor.use_smaa,
            descriptor.sample_count,
        );
        let sample_count = multisampling_strategy.get_msaa_sample_count();

        let blend = descriptor
            .blend_state
            .or(Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING));

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                compilation_options: Default::default(),
                buffers: &[vertex_buffer_layout],
            },
            primitive: PrimitiveState {
                cull_mode: None,
                front_face: FrontFace::Cw,
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: Self::DEPTH_TEXTURE_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: MultisampleState {
                count: sample_count,
                alpha_to_coverage_enabled: false,
                ..Default::default()
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: texture_format,
                    blend,
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
            texture_format,
            adapter,
            pipeline,
            multisampling_strategy,
            layouts: GraphicsContextLayouts {
                pipeline_layout,
                transform_bind_group_layout,
                skin_sampler_bind_group_layout: skin_bind_group_layout,
                sun_bind_group_layout,
            },
        })
    }

    pub fn set_surface_size(&mut self, size: Size) {
        if let Ok(Some(config)) = &mut self.surface_config {
            config.width = size.width;
            config.height = size.height;

            if let Some(surface) = &mut self.surface {
                surface.configure(&self.device, config);
            }
        }
    }
}
