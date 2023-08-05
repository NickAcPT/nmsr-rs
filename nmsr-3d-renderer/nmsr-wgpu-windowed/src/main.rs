use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{iter, mem};

use egui::emath::Numeric;
use egui::{Context, FontDefinitions};
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use libloader::libloading;
use nmsr_rendering::errors::NMSRRenderingError;
use nmsr_rendering::high_level::pipeline::scene::{self, Scene, Size};
use nmsr_rendering::high_level::pipeline::{
    GraphicsContext, GraphicsContextDescriptor, SceneContext,
};
use strum::IntoEnumIterator;
use wgpu::util::DeviceExt;
use wgpu::{
    include_wgsl, Backends, BufferAddress, Device, Instance, RenderPassDepthStencilAttachment,
    SurfaceConfiguration, Texture, TextureView,
};
use winit::event;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;

use nmsr_player_parts::parts::part::Part;
use nmsr_player_parts::parts::provider::minecraft::MinecraftPlayerPartsProvider;
use nmsr_player_parts::parts::provider::{PartsProvider, PlayerPartProviderContext};
use nmsr_player_parts::parts::uv::FaceUv;
use nmsr_player_parts::player_model::PlayerModel;
use nmsr_player_parts::types::PlayerBodyPartType;
use nmsr_rendering::high_level::camera::{
    Camera, CameraPositionParameters, CameraRotation, ProjectionParameters,
};
use nmsr_rendering::low_level::primitives::cube::Cube;
use nmsr_rendering::low_level::primitives::mesh::Mesh;
use nmsr_rendering::low_level::primitives::part_primitive::PartPrimitive;
use nmsr_rendering::low_level::primitives::vertex::Vertex;
use nmsr_rendering::low_level::{Vec2, Vec3};
use winit::platform::run_return::EventLoopExtRunReturn;

#[tokio::main]
async fn main() -> Result<(), NMSRRenderingError> {
    mem::forget(unsafe {
        libloading::Library::new("D:\\IDEs\\CLionProjects\\nmsr-wgpu\\vulkan-1.dll").unwrap()
    });

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
    println!("Window size: {}x{}", size.width, size.height);

    let graphics_context = GraphicsContext::new(GraphicsContextDescriptor {
        backends: Some(wgpu::Backends::all()),
        surface_provider: Box::new(|i: &Instance| unsafe {
            Some(i.create_surface(&window).unwrap())
        }),
        default_size: (size.width, size.height),
        texture_format: None,
    })
    .await
    .expect("Expected Nmsr Pipeline");

    let instance = &graphics_context.instance;
    instance.enumerate_adapters(Backends::all()).for_each(|d| {
        println!("Adapter: {}", d.get_info().name);
    });

    let surface_view_format = graphics_context
        .surface_view_format
        .expect("Expected surface view format");

    {
        let adapter = &graphics_context.adapter;
        let adapter_info = adapter.get_info();
        println!("Using {} ({:?})", adapter_info.name, adapter_info.backend);
    }

    let (width, height) = {
        let config = &graphics_context
            .surface_config
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .read()
            .unwrap();

        (config.width, config.height)
    };

    let mut camera = Camera::new_absolute(
        Vec3::new(0.0, 30.0, -20.0),
        CameraRotation {
            yaw: 0.0,
            pitch: 0.0,
        },
        ProjectionParameters::Perspective { fov: 110f32 },
        1.0,
    );

    
    let mut has_captured_frame = false;
    
    if !has_captured_frame {
        renderdoc.start_frame_capture(std::ptr::null(), std::ptr::null())
    }
    
    
    let graphics_context = Arc::new(graphics_context);

    let scene_context = Arc::new(SceneContext::new(Arc::clone(&graphics_context)));

    let scene = Scene::new(scene_context.clone(), camera, Size { width, height });

    let mut camera = scene.camera;

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
    let (vertex_data, index_data) = (to_render.get_vertices(), to_render.get_indices());

    let vertex_buf = graphics_context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertex_data),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buf = graphics_context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Index Buffer"),
        contents: bytemuck::cast_slice(&index_data),
        usage: wgpu::BufferUsages::INDEX,
    });

    // Create pipeline layout
    let skin_bind_group_layout = &graphics_context.skin_bind_group_layout;

    let skin_bytes =
        include_bytes!("819ba7dd7373fb71c763ac3ce0fe976a0acd16d4f7bc56d6b9c198e4bc379981.png");
    let skin_image = image::load_from_memory(skin_bytes).unwrap();
    let mut skin_rgba = skin_image.to_rgba8();

    ears_rs::utils::alpha::strip_alpha(&mut skin_rgba);

    let skin_texture = graphics_context.device.create_texture(&wgpu::TextureDescriptor {
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

    graphics_context.queue.write_texture(
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

    let skin_texture_view = skin_texture.create_view(&Default::default());

    let skin_bind_group = graphics_context.device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: skin_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&skin_texture_view),
        }],
        label: Some("diffuse_bind_group"),
    });

    //EGUI:let mut egui_rpass = RenderPass::new(&device, surface_view_format, 1);

    //EGUI:let mut platform = Platform::new(PlatformDescriptor {
    //EGUI:    physical_width: width,
    //EGUI:    physical_height: height,
    //EGUI:    scale_factor: window.scale_factor(),
    //EGUI:    font_definitions: FontDefinitions::default(),
    //EGUI:    style: Default::default(),
    //EGUI:});

    let (mut depth_texture, mut depth) = create_depth(
        &graphics_context.device,
        &graphics_context
            .surface_config
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .read()
            .unwrap(),
    );

    println!("Entering render loop...");
    let start_time = Instant::now();
    let mut last_frame_time = Duration::ZERO;

    let mut last_camera_stuff: Option<(CameraPositionParameters, CameraRotation)> = None;

    event_loop.run(move |event, _, control_flow| {
        let graphics_context = graphics_context.clone();
        
        //EGUI:platform.handle_event(&event);

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
                let adapter = &graphics_context.adapter;

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
                    let size = Size {
                        width: size.width.max(1),
                        height: size.height.max(1),
                    };

                    graphics_context.set_surface_size(size);

                    camera.set_aspect_ratio(size.width as f32 / size.height as f32);

                    (depth_texture, depth) = create_depth(
                        &graphics_context.device,
                        &graphics_context
                            .surface_config
                            .as_ref()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .read()
                            .unwrap(),
                    )
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
                            camera.set_position_z(camera.get_position_z() + 0.5);
                        }
                        Some(event::VirtualKeyCode::S) => {
                            camera.set_position_z(camera.get_position_z() - 0.5);
                        }
                        Some(event::VirtualKeyCode::A) => {
                            camera.set_position_x(camera.get_position_x() + 0.5);
                        }
                        Some(event::VirtualKeyCode::D) => {
                            camera.set_position_x(camera.get_position_x() - 0.5);
                        }
                        Some(event::VirtualKeyCode::Q) => {
                            camera.set_position_y(camera.get_position_y() + 0.5);
                        }
                        Some(event::VirtualKeyCode::E) => {
                            camera.set_position_y(camera.get_position_y() - 0.5);
                        }
                        Some(event::VirtualKeyCode::V) => {
                            visage_orbital(&mut camera, &mut last_camera_stuff);
                        }
                        // R
                        Some(event::VirtualKeyCode::R) => {
                            //println!("Triggering RenderDoc capture.");
                            println!("Last frame time: {:?}", last_frame_time);
                            has_captured_frame = false;
                        }
                        _ => {}
                    }
                }
            }
            event::Event::RedrawRequested(_) => {
                let surface = graphics_context
                    .surface
                    .as_ref()
                    .expect("Expected surface")
                    .read()
                    .unwrap();
                //EGUI:platform.update_time(start_time.elapsed().as_secs_f64());
                let start = Instant::now();

                let frame = match surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(_) => {
                        surface.configure(
                            &graphics_context.device,
                            &graphics_context
                                .surface_config
                                .as_ref()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .read()
                                .unwrap(),
                        );
                        surface
                            .get_current_texture()
                            .expect("Failed to acquire next surface texture!")
                    }
                };

                let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
                    format: Some(surface_view_format),
                    ..Default::default()
                });

                graphics_context.device.push_error_scope(wgpu::ErrorFilter::Validation);

                let queue = &graphics_context.queue;
                
                let mut encoder =
                    graphics_context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Main Encoder") });
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
                    rpass.set_pipeline(&graphics_context.pipeline);
                    rpass.set_bind_group(0, &scene_context.transform_bind_group, &[]);
                    rpass.set_bind_group(1, &skin_bind_group, &[]);
                    rpass.set_index_buffer(index_buf.slice(..), wgpu::IndexFormat::Uint16);
                    rpass.set_vertex_buffer(0, vertex_buf.slice(..));
                    rpass.pop_debug_group();
                    rpass.insert_debug_marker("Draw!");
                    rpass.draw_indexed(0..(index_data.len() as u32), 0, 0..1);
                }

                queue.submit(Some(encoder.finish()));

                // Begin to draw the UI frame.

                //EGUI:platform.begin_frame();

                // Draw the demo application.
                //EGUI:{
                //EGUI:    debug_ui(&platform.context(), &mut camera, &mut last_camera_stuff, last_frame_time);
                //EGUI:}

                // End the UI frame. We could now handle the output and draw the UI with the backend.
                //EGUI:let full_output = platform.end_frame(Some(&window));
                //EGUI:let paint_jobs = platform.context().tessellate(full_output.shapes);

                //EGUI:let mut encoder = graphics_context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                //EGUI:    label: Some("Egui encoder"),
                //EGUI:});

                // Upload all resources for the GPU.
                //EGUI:let screen_descriptor = ScreenDescriptor {
                //EGUI:    physical_width: width,
                //EGUI:    physical_height: height,
                //EGUI:    scale_factor: window.scale_factor() as f32,
                //EGUI:};
                //EGUI:let tdelta: egui::TexturesDelta = full_output.textures_delta;
                //EGUI:egui_rpass
                //EGUI:    .add_textures(&device, &graphics_context.queue, &tdelta)
                //EGUI:    .expect("add texture ok");
                //EGUI:egui_rpass.update_buffers(&device, &graphics_context.queue, &paint_jobs, &screen_descriptor);

                // Record all render passes.
                //EGUI:egui_rpass
                //EGUI:.execute(&mut encoder, &view, &paint_jobs, &screen_descriptor, None)
                //EGUI:.unwrap();

                // Submit the commands.
                //EGUI:graphics_context.queue.submit(iter::once(encoder.finish()));

                //EGUI:egui_rpass
                //EGUI:.remove_textures(tdelta)
                //EGUI:.expect("remove texture ok");

                frame.present();

                last_frame_time = start.elapsed();

                let mx_total = camera.get_view_projection_matrix();
                let mx_ref: &[f32; 16] = mx_total.as_ref();
                graphics_context
                    .queue
                    .write_buffer(&scene_context.transform_matrix_buffer, 0, bytemuck::cast_slice(mx_ref));
                
                if !has_captured_frame {
                    has_captured_frame = true;
                    renderdoc.end_frame_capture(std::ptr::null(), std::ptr::null());
                }
            }
            _ => {}
        }
    });
}

fn create_depth(device: &Device, config: &SurfaceConfiguration) -> (Texture, TextureView) {
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
    let depth = depth_texture.create_view(&Default::default());
    (depth_texture, depth)
}

fn debug_ui(
    ctx: &Context,
    camera: &mut Camera,
    last_camera_stuff: &mut Option<(CameraPositionParameters, CameraRotation)>,
    last_frame_time: Duration,
) {
    egui::Window::new("Camera").vscroll(true).show(ctx, |ui| {
        ui.label(format!("Last Frame time: {:?}", last_frame_time));

        ui.horizontal(|ui| {
            if ui.button("Visage").clicked() {
                visage(camera, last_camera_stuff);
            }
            if ui.button("Visage (Orbital)").clicked() {
                visage_orbital(camera, last_camera_stuff);
            }

            if last_camera_stuff.is_some() && ui.button("Last").clicked() {
                let current = (camera.get_position_parameters(), camera.get_rotation());

                let (position, rotation) = last_camera_stuff.unwrap();
                camera.set_position_parameters(position);
                camera.set_rotation(rotation);

                last_camera_stuff.replace(current);
            }
        });

        ui.label("Camera");

        {
            let position_params = camera.get_position_parameters_as_mut();

            ui.label("Position Parameters");
            ui.horizontal(|ui| {
                ui.radio_value(
                    position_params,
                    CameraPositionParameters::Absolute(Vec3::new(0.0, 30.0, -20.0)),
                    "Absolute",
                );
                ui.radio_value(
                    position_params,
                    CameraPositionParameters::Orbital {
                        look_at: Vec3::new(0.0, 20.0, 0.0),
                        distance: 20.0,
                    },
                    "Orbital",
                );
            });
        }

        if let CameraPositionParameters::Absolute(_) = camera.get_position_parameters() {
            ui.label("X");
            ui.add(drag_value(
                camera,
                Camera::get_position_x,
                Camera::set_position_x,
                None,
                None,
            ));
            ui.label("Y");
            ui.add(drag_value(
                camera,
                Camera::get_position_y,
                Camera::set_position_y,
                None,
                None,
            ));
            ui.label("Z");
            ui.add(drag_value(
                camera,
                Camera::get_position_z,
                Camera::set_position_z,
                None,
                None,
            ));
        }
        if let CameraPositionParameters::Orbital { .. } = camera.get_position_parameters() {
            ui.label("Look At: X");
            ui.add(drag_value(
                camera,
                Camera::get_look_at_x,
                Camera::set_look_at_x,
                None,
                None,
            ));
            ui.label("Look At: Y");
            ui.add(drag_value(
                camera,
                Camera::get_look_at_y,
                Camera::set_look_at_y,
                None,
                None,
            ));
            ui.label("Look At: Z");
            ui.add(drag_value(
                camera,
                Camera::get_look_at_z,
                Camera::set_look_at_z,
                None,
                None,
            ));

            ui.label("Distance");
            ui.add(drag_value(
                camera,
                Camera::get_distance,
                Camera::set_distance,
                Some(0.0f32),
                None,
            ));
        }

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

        ui.label("Projection");
        ui.horizontal(|ui| {
            ui.radio_value(
                projection,
                ProjectionParameters::Perspective { fov: 110f32 },
                "Perspective",
            );
            ui.radio_value(
                projection,
                ProjectionParameters::Orthographic { aspect: 15.0f32 },
                "Orthographic",
            );
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

fn visage_orbital(
    camera: &mut Camera,
    last_camera_stuff: &mut Option<(CameraPositionParameters, CameraRotation)>,
) {
    last_camera_stuff.replace((camera.get_position_parameters(), camera.get_rotation()));

    camera.set_position_parameters(CameraPositionParameters::Orbital {
        look_at: [0.0, 16.65, 0.0].into(),
        distance: 44.1,
    });

    camera.set_projection(ProjectionParameters::Perspective { fov: 45.0 });

    camera.set_rotation(CameraRotation {
        yaw: 20.0,
        pitch: 10.0,
    })
}

fn visage(
    camera: &mut Camera,
    last_camera_stuff: &mut Option<(CameraPositionParameters, CameraRotation)>,
) {
    last_camera_stuff.replace((camera.get_position_parameters(), camera.get_rotation()));

    camera.set_position_parameters(CameraPositionParameters::Absolute(Vec3::new(
        14.85, 24.3, -40.85,
    )));

    camera.set_projection(ProjectionParameters::Perspective { fov: 45.0 });

    camera.set_rotation(CameraRotation {
        yaw: 20.0,
        pitch: 10.0,
    })
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
    let small_offset = 1f32 / 16f32 / 64f32;
    top_left += small_offset;
    bottom_right -= small_offset;
    [top_left, bottom_right]
}
