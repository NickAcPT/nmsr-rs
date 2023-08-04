use std::borrow::Cow;
use std::mem::{self, size_of};
use std::sync::Arc;

use glam::{Mat4, Vec3};
use wgpu::util::{DeviceExt, BufferInitDescriptor};
use wgpu::BufferBindingType::Uniform;
use wgpu::{BindGroupLayoutEntry, BindingType, BufferAddress, ShaderStages, TextureFormat, DepthStencilState, CompareFunction, BlendState, ColorWrites, ColorTargetState, FragmentState, RenderPipelineDescriptor, VertexBufferLayout, ShaderModuleDescriptor, BindGroupDescriptor, PipelineLayoutDescriptor, BindGroupLayoutDescriptor};

use crate::high_level::pipeline::graphics_context::GraphicsContext;
use crate::low_level::primitives::vertex::Vertex;

#[derive(Debug)]
pub struct SceneContext {
    context: Arc<GraphicsContext>,
    pipeline: wgpu::RenderPipeline,
    transform_bind_group: wgpu::BindGroup,
    transform_matrix_buffer: wgpu::Buffer,
}

impl SceneContext {
    const TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba8Uint;

    pub fn new(context: Arc<GraphicsContext>) -> Self {
        let device = &context.device;

        // Create a bind group layout for storing the transformation matrix in a uniform
        let transform_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(64),
                    },
                    count: None,
                }],
                label: Some("Transform Bind Group Layout"),
            });

        // Create the pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Scene Pipeline Layout"),
            bind_group_layouts: &[&transform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let transform_matrix_buffer =
            device.create_buffer_init(&BufferInitDescriptor {
                label: Some("Transform Matrix Buffer"),
                contents: bytemuck::cast_slice(Mat4::IDENTITY.as_ref()),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let transform_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Transform Bind Group"),
            layout: &transform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: transform_matrix_buffer.as_entire_binding(),
            }],
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
                    format: Self::TEXTURE_FORMAT,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });
        
        Self {
            context,
            pipeline,
            transform_bind_group,
            transform_matrix_buffer
        }
    }
}
