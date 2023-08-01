use std::borrow::Cow;
use std::time::Instant;
use std::{iter, mem};

use egui::emath::Numeric;
use egui::{Context, FontDefinitions};
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use strum::IntoEnumIterator;
use wgpu::util::DeviceExt;
use wgpu::{BufferAddress, DepthBiasState, Instance, RenderPassDepthStencilAttachment};
use winit::event;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;

use nmsr_player_parts::parts::minecraft::MinecraftPlayerPartsProvider;
use nmsr_player_parts::parts::part::Part;
use nmsr_player_parts::parts::player_model::PlayerModel;
use nmsr_player_parts::parts::provider::{PartsProvider, PlayerPartProviderContext};
use nmsr_player_parts::parts::types::PlayerBodyPartType;
use nmsr_player_parts::parts::uv::FaceUv;
use nmsr_rendering::high_level::camera::{Camera, CameraRotation, ProjectionParameters};
use nmsr_rendering::high_level::errors::NMSRRenderingError;
use nmsr_rendering::high_level::pipeline::wgpu_pipeline::{
    NmsrPipelineDescriptor, NmsrWgpuPipeline,
};
use nmsr_rendering::low_level::primitives::cube::Cube;
use nmsr_rendering::low_level::primitives::mesh::Mesh;
use nmsr_rendering::low_level::primitives::part_primitive::PartPrimitive;
use nmsr_rendering::low_level::primitives::vertex::Vertex;
use nmsr_rendering::low_level::{Vec2, Vec3};

#[tokio::main]
async fn main() -> Result<(), NMSRRenderingError> {
    let mut renderdoc =
        renderdoc::RenderDoc::<renderdoc::V140>::new().expect("Failed to initialize RenderDoc");

    renderdoc
        .launch_replay_ui(true, None)
        .expect("Failed to launch RenderDoc replay UI");

    let event_loop = EventLoop::new();
    let mut builder = winit::window::WindowBuilder::new();
    builder = builder.with_title("NMSR WGPU Windowed");
    let window = builder.build(&event_loop).unwrap();

    let size = window.inner_size();

    let pipeline = NmsrWgpuPipeline::new(NmsrPipelineDescriptor {
        backends: Some(wgpu::Backends::all()),
        surface_provider: Box::new(|i: &Instance| unsafe {
            Some(i.create_surface(&window).unwrap())
        }),
        default_size: (size.width, size.height),
    })
    .await
    .expect("Expected Nmsr Pipeline");

    let device = pipeline.device;
    let queue = pipeline.queue;
    let surface = pipeline.surface.expect("Expected surface");
    let mut config = pipeline.surface_config.expect("Expected surface config")?;
    let adapter = pipeline.adapter;
    let surface_view_format = pipeline
        .surface_view_format
        .expect("Expected surface view format");

    let adapter_info = adapter.get_info();
    println!("Using {} ({:?})", adapter_info.name, adapter_info.backend);

    let uv = Vec2::new(0.0, 0.0);
    let uv2 = Vec2::new(1.0, 1.0);

    let aspect_ratio = config.width as f32 / config.height as f32;

    let mut camera = Camera::new(
        Vec3::new(0.0, 30.0, -20.0),
        CameraRotation {
            yaw: 0.0,
            pitch: 0.0,
        },
        ProjectionParameters::Perspective {
            fov: 110f32
        },
        aspect_ratio,
    );

    let ctx = PlayerPartProviderContext {
        model: PlayerModel::Alex,
    };

    let to_render: Vec<_> = PlayerBodyPartType::iter()
        .flat_map(|part| MinecraftPlayerPartsProvider.get_parts(&ctx, part))
        .map(primitive_convert)
        .collect();

    println!("To render count: {}", to_render.len());

    let to_render = Mesh::new(to_render);

    // Create the vertex and index buffers
    let vertex_size = mem::size_of::<Vertex>();
    let (vertex_data, index_data) = (to_render.get_vertices(), to_render.get_indices());

    let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertex_data),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Index Buffer"),
        contents: bytemuck::cast_slice(&index_data),
        usage: wgpu::BufferUsages::INDEX,
    });

    // Create pipeline layout
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(64),
            },
            count: None,
        }],
    });

    let skin_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    // This should match the filterable field of the
                    // corresponding Texture entry above.
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout, &skin_bind_group_layout],
        push_constant_ranges: &[],
    });

    let mx_total = camera.get_view_projection_matrix();
    let mx_ref: &[f32; 16] = mx_total.as_ref();
    let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Uniform Buffer"),
        contents: bytemuck::cast_slice(mx_ref),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    // Create bind group
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buf.as_entire_binding(),
        }],
        label: None,
    });

    let skin_bytes = include_bytes!("ears_v0_sample1.png");
    let skin_image = image::load_from_memory(skin_bytes).unwrap();
    let mut skin_rgba = skin_image.to_rgba8();

    ears_rs::utils::alpha::strip_alpha(&mut skin_rgba);

    let skin_texture = device.create_texture(&wgpu::TextureDescriptor {
        // All textures are stored as 3D, we represent our 2D texture
        // by setting depth to 1.
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // Most images are stored using sRGB so we need to reflect that here.
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        // TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
        // COPY_DST means that we want to copy data to this texture
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some("skin_texture"),
        // This is the same as with the SurfaceConfig. It
        // specifies what texture formats can be used to
        // create TextureViews for this texture. The base
        // texture format (Rgba8UnormSrgb in this case) is
        // always supported. Note that using a different
        // texture format is not supported on the WebGL2
        // backend.
        view_formats: &[],
    });

    queue.write_texture(
        // Tells wgpu where to copy the pixel data
        wgpu::ImageCopyTexture {
            texture: &skin_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        // The actual pixel data
        &skin_rgba,
        // The layout of the texture
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * 64),
            rows_per_image: Some(64),
        },
        wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
    );

    let skin_texture_view = skin_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let skin_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
    });

    let vertex_buffers = [wgpu::VertexBufferLayout {
        array_stride: vertex_size as BufferAddress,
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
    }];

    let skin_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &skin_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&skin_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&skin_sampler),
            },
        ],
        label: Some("diffuse_bind_group"),
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &vertex_buffers,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: config.view_formats[0],
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: None, //Some(wgpu::Face::Back),
            front_face: wgpu::FrontFace::Cw,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: Default::default(),
            bias: DepthBiasState {
                constant: 5,
                slope_scale: 2.0,
                clamp: 20.0,
            }
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let mut egui_rpass = RenderPass::new(&device, surface_view_format, 1);

    let mut platform = Platform::new(PlatformDescriptor {
        physical_width: config.width,
        physical_height: config.height,
        scale_factor: window.scale_factor(),
        font_definitions: FontDefinitions::default(),
        style: Default::default(),
    });

    println!("Entering render loop...");
    let start_time = Instant::now();
    event_loop.run(move |event, _, control_flow| {
        platform.handle_event(&event);

        match event {
            event::Event::RedrawEventsCleared => {
                window.request_redraw();
            }
            event::Event::WindowEvent {
                event:
                    WindowEvent::Resized(size)
                    | WindowEvent::ScaleFactorChanged {
                        new_inner_size: &mut size,
                        ..
                    },
                ..
            } => {
                // Once winit is fixed, the detection conditions here can be removed.
                // https://github.com/rust-windowing/winit/issues/2876
                let max_dimension = adapter.limits().max_texture_dimension_2d;
                if size.width > max_dimension || size.height > max_dimension {
                    println!(
                        "The resizing size {:?} exceeds the limit of {}.",
                        size, max_dimension
                    );
                } else {
                    println!("Resizing to {:?}", size);
                    config.width = size.width.max(1);
                    config.height = size.height.max(1);
                    surface.configure(&device, &config);
                }
            }
            event::Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = winit::event_loop::ControlFlow::Exit;
            }
            // On keyboard input, move the camera
            // W is forward, S is backward, A is left, D is right, Q is up, E is down
            // We are facing South
            event::Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                ..
            } => {
                if input.state == event::ElementState::Pressed {
                    match input.virtual_keycode {
                        Some(event::VirtualKeyCode::W) => {
                            camera.set_z(camera.get_z() + 0.5);
                        }
                        Some(event::VirtualKeyCode::S) => {
                            camera.set_z(camera.get_z() - 0.5);
                        }
                        Some(event::VirtualKeyCode::A) => {
                            camera.set_x(camera.get_x() + 0.5);
                        }
                        Some(event::VirtualKeyCode::D) => {
                            camera.set_x(camera.get_x() - 0.5);
                        }
                        Some(event::VirtualKeyCode::Q) => {
                            camera.set_y(camera.get_y() + 0.5);
                        }
                        Some(event::VirtualKeyCode::E) => {
                            camera.set_y(camera.get_y() - 0.5);
                        }
                        // R
                        Some(event::VirtualKeyCode::R) => {
                            println!("Triggering RenderDoc capture.");
                            renderdoc.trigger_capture();
                        }
                        _ => {}
                    }
                }
            }
            event::Event::RedrawRequested(_) => {
                platform.update_time(start_time.elapsed().as_secs_f64());

                let frame = match surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(_) => {
                        surface.configure(&device, &config);
                        surface
                            .get_current_texture()
                            .expect("Failed to acquire next surface texture!")
                    }
                };
                let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
                    format: Some(surface_view_format),
                    ..wgpu::TextureViewDescriptor::default()
                });

                let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
                    size: wgpu::Extent3d {
                        width: config.width,
                        height: config.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Depth32Float,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    label: None,
                    view_formats: &[],
                });
                let depth = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

                device.push_error_scope(wgpu::ErrorFilter::Validation);

                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Main render pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.1,
                                    g: 0.2,
                                    b: 0.3,
                                    a: 1.0,
                                }),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                            view: &depth,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: true,
                            }),
                            stencil_ops: None,
                        }),
                    });

                    rpass.push_debug_group("Prepare data for draw.");
                    rpass.set_pipeline(&pipeline);
                    rpass.set_bind_group(0, &bind_group, &[]);
                    rpass.set_bind_group(1, &skin_bind_group, &[]);
                    rpass.set_index_buffer(index_buf.slice(..), wgpu::IndexFormat::Uint16);
                    rpass.set_vertex_buffer(0, vertex_buf.slice(..));
                    rpass.pop_debug_group();
                    rpass.insert_debug_marker("Draw!");
                    rpass.draw_indexed(0..(index_data.len() as u32), 0, 0..1);
                }

                queue.submit(Some(encoder.finish()));

                // Begin to draw the UI frame.
                platform.begin_frame();

                // Draw the demo application.
                {
                    debug_ui(&platform.context(), &mut camera);
                }

                // End the UI frame. We could now handle the output and draw the UI with the backend.
                let full_output = platform.end_frame(Some(&window));
                let paint_jobs = platform.context().tessellate(full_output.shapes);

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("encoder"),
                });

                // Upload all resources for the GPU.
                let screen_descriptor = ScreenDescriptor {
                    physical_width: config.width,
                    physical_height: config.height,
                    scale_factor: window.scale_factor() as f32,
                };
                let tdelta: egui::TexturesDelta = full_output.textures_delta;
                egui_rpass
                    .add_textures(&device, &queue, &tdelta)
                    .expect("add texture ok");
                egui_rpass.update_buffers(&device, &queue, &paint_jobs, &screen_descriptor);

                // Record all render passes.
                egui_rpass
                    .execute(&mut encoder, &view, &paint_jobs, &screen_descriptor, None)
                    .unwrap();
                // Submit the commands.
                queue.submit(iter::once(encoder.finish()));

                egui_rpass
                    .remove_textures(tdelta)
                    .expect("remove texture ok");

                frame.present();

                let mx_total = camera.get_view_projection_matrix();
                let mx_ref: &[f32; 16] = mx_total.as_ref();
                queue.write_buffer(&uniform_buf, 0, bytemuck::cast_slice(mx_ref));
            }
            _ => {}
        }
    });
}

fn debug_ui(ctx: &Context, camera: &mut Camera) {
    egui::Window::new("Camera").vscroll(true).show(ctx, |ui| {
        ui.label("Camera");
        ui.label("X");
        ui.add(drag_value(camera, Camera::get_x, Camera::set_x, None, None));
        ui.label("Y");
        ui.add(drag_value(camera, Camera::get_y, Camera::set_y, None, None));
        ui.label("Z");
        ui.add(drag_value(camera, Camera::get_z, Camera::set_z, None, None));
        ui.label("Yaw");
        ui.add(drag_value(
            camera,
            Camera::get_yaw,
            Camera::set_yaw,
            Some(-180.0f32),
            Some(180.0f32),
        ));
        ui.label("Pitch");
        ui.add(drag_value(
            camera,
            Camera::get_pitch,
            Camera::set_pitch,
            Some(-90.0f32),
            Some(90.0f32),
        ));

        ui.separator();

        // ProjectionParameters enum (enum variant takes in fov or aspect) { Perspective {fov: f32 }, Orthographic {aspect: f32} }
        let projection = camera.get_projection_as_mut();

        // Create a combo box with the enum variants
        // Then show the appropriate fields for the selected variant

        ui.vertical(|ui| {
            ui.label("Projection");
            ui.radio_value(projection, ProjectionParameters::Perspective { fov: 110f32 }, "Perspective");
            ui.radio_value(projection, ProjectionParameters::Orthographic { aspect: 15.0f32 }, "Orthographic");

        });

        if let ProjectionParameters::Perspective { .. } = projection {
            ui.label("FOV");
            ui.add(drag_value(
                camera,
                Camera::get_fov,
                Camera::set_fov,
                None,
                None,
            ));
        } else if let ProjectionParameters::Orthographic { .. } = projection {
            ui.label("Aspect");
            ui.add(drag_value(
                camera,
                Camera::get_aspect,
                Camera::set_aspect,
                None,
                None,
            ));
        }
    });
}

fn drag_value<T, I>(
    value: &mut I,
    get: fn(&I) -> T,
    set: fn(&mut I, T),
    min: Option<T>,
    max: Option<T>,
) -> egui::DragValue
where
    T: Numeric,
{
    let value = egui::DragValue::from_get_set(move |new| {
        if let Some(new) = new {
            set(value, Numeric::from_f64(new));
        }

        get(value).to_f64()
    });

    let min = min.unwrap_or(T::MIN);
    let max = max.unwrap_or(T::MAX);

    value
        .clamp_range(min.to_f64()..=max.to_f64())
        .speed(0.25)
        .max_decimals(1)
}

fn primitive_convert(part: Part) -> Box<dyn PartPrimitive> {
    Box::new(match part {
        Part::Cube {
            position,
            size,
            face_uvs,
            ..
        } => {
            // Compute center of cube
            let center = position + size / 2.0;

            Cube::new(
                center,
                size,
                uv(&face_uvs.north),
                uv(&face_uvs.south),
                uv(&face_uvs.up),
                uv(&face_uvs.down),
                uv(&face_uvs.west),
                uv(&face_uvs.east),
            )
        }
        Part::Quad { .. } => {
            unreachable!()
        }
    })
}

fn uv(face_uvs: &FaceUv) -> [Vec2; 2] {
    let mut top_left = face_uvs.top_left.to_uv([64f32, 64f32].into());
    let mut bottom_right = face_uvs.bottom_right.to_uv([64f32, 64f32].into());
    let small_offset= 1f32 / 16f32 / 64f32;
    top_left += small_offset;
    bottom_right -= small_offset;
    [
        top_left,
        bottom_right,
    ]
}
