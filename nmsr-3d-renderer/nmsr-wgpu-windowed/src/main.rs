use std::borrow::Borrow;
use std::iter;
use std::time::{Duration, Instant};

use egui::emath::Numeric;
use egui::{Context, FontDefinitions};
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};

use nmsr_rendering::high_level::pipeline::scene::{self, Scene};
use nmsr_rendering::high_level::pipeline::{
    GraphicsContext, GraphicsContextDescriptor, SceneContext,
};
use nmsr_rendering::low_level::{Vec2, Vec3};
use strum::IntoEnumIterator;

use wgpu::{
    Backends, Device, Instance, SurfaceConfiguration, Texture, TextureDescriptor, TextureUsages,
    TextureView,
};
use winit::event;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;

use nmsr_player_parts::parts::part::Part;
use nmsr_player_parts::parts::provider::PlayerPartProviderContext;
use nmsr_player_parts::parts::uv::FaceUv;
use nmsr_player_parts::player_model::PlayerModel;
use nmsr_player_parts::types::PlayerBodyPartType;
use nmsr_rendering::high_level::camera::{
    Camera, CameraPositionParameters, CameraRotation, ProjectionParameters,
};
use nmsr_rendering::low_level::primitives::cube::Cube;
use nmsr_rendering::low_level::primitives::part_primitive::PartPrimitive;
use winit::platform::run_return::EventLoopExtRunReturn;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    //mem::forget(unsafe {
    //    libloading::Library::new("D:\\IDEs\\CLionProjects\\nmsr-wgpu\\vulkan-1.dll").unwrap()
    //});

    let mut renderdoc =
        renderdoc::RenderDoc::<renderdoc::V140>::new().expect("Failed to initialize RenderDoc");

    renderdoc
        .launch_replay_ui(true, None)
        .expect("Failed to launch RenderDoc replay UI");

    let mut event_loop = EventLoop::new();
    let mut builder = winit::window::WindowBuilder::new();
    builder = builder.with_title("NMSR WGPU Windowed");
    let window = builder.build(&event_loop).unwrap();

    let size = window.inner_size();

    let graphics = GraphicsContext::new(GraphicsContextDescriptor {
        backends: Some(wgpu::Backends::all()),
        surface_provider: Box::new(|i: &Instance| unsafe {
            Some(i.create_surface(&window).unwrap())
        }),
        default_size: (size.width, size.height),
        texture_format: None,
    })
    .await
    .expect("Expected Nmsr Pipeline");

    let instance = &graphics.instance;
    instance.enumerate_adapters(Backends::all()).for_each(|a| {
        println!("Adapter: {}", a.get_info().name);
    });

    let device = &graphics.device;
    let surface = graphics.surface.as_ref().expect("Expected surface");
    let config = graphics
        .surface_config
        .as_ref()
        .ok()
        .unwrap()
        .as_ref()
        .expect("OwO");

    let adapter = &graphics.adapter;
    let surface_view_format = &graphics.texture_format;

    let adapter_info = adapter.get_info();
    println!("Using {} ({:?})", adapter_info.name, adapter_info.backend);

    let aspect_ratio = config.width as f32 / config.height as f32;

    let camera = Camera::new_absolute(
        Vec3::new(0.0, 30.0, -20.0),
        CameraRotation {
            yaw: 0.0,
            pitch: 0.0,
        },
        ProjectionParameters::Perspective { fov: 110f32 },
        aspect_ratio,
    );

    let ctx = PlayerPartProviderContext {
        model: PlayerModel::Alex,
    };

    let mut scene = Scene::new(
        &graphics,
        SceneContext::new(&graphics),
        camera,
        scene::Size {
            width: config.width,
            height: config.height,
        },
        &ctx,
        PlayerBodyPartType::iter(),
    );

    // Create pipeline layout
    let skin_bytes =
        include_bytes!("819ba7dd7373fb71c763ac3ce0fe976a0acd16d4f7bc56d6b9c198e4bc379981.png");
    let skin_image = image::load_from_memory(skin_bytes).unwrap();
    let mut skin_rgba = skin_image.to_rgba8();

    ears_rs::utils::alpha::strip_alpha(&mut skin_rgba);

    // Upload skin
    scene.set_texture(
        &graphics,
        nmsr_player_parts::types::PlayerPartTextureType::Skin,
        &skin_rgba,
    );

    println!("surface_view_format: {:?}", surface_view_format);

    println!("Entering render loop...");
    let start_time = Instant::now();
    let mut last_frame_time = Duration::ZERO;

    let mut last_camera_stuff: Option<(CameraPositionParameters, CameraRotation)> = None;

    let mut egui_rpass = RenderPass::new(device, *surface_view_format, 1);

    let mut platform = Platform::new(PlatformDescriptor {
        physical_width: config.width,
        physical_height: config.height,
        scale_factor: window.scale_factor(),
        font_definitions: FontDefinitions::default(),
        style: Default::default(),
    });

    event_loop.run_return(|event, _, control_flow| {
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
                    let camera = scene.camera_mut();
                    let mut new_config = config.clone();

                    new_config.width = size.width.max(1);
                    new_config.height = size.height.max(1);
                    surface.configure(device, &new_config);
                    camera.set_aspect_ratio(new_config.width as f32 / new_config.height as f32);

                    scene.viewport_size_mut().width = new_config.width;
                    scene.viewport_size_mut().height = new_config.height;
                    scene.update(&graphics)
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
                    let camera = scene.camera_mut();
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
                            visage_orbital(camera, &mut last_camera_stuff);
                        }
                        // R
                        Some(event::VirtualKeyCode::R) => {
                            //println!("Triggering RenderDoc capture.");
                            println!("Last frame time: {:?}", last_frame_time);
                            renderdoc.trigger_capture();
                        }
                        _ => {}
                    }
                }
            }
            event::Event::RedrawRequested(_) => {
                let start = Instant::now();

                scene
                    .render_with_extra(
                        &graphics,
                        Some(Box::new(|view, enc, camera| {
                            platform.update_time(start_time.elapsed().as_secs_f64());

                            platform.begin_frame();

                            {
                                debug_ui(
                                    &platform.context(),
                                    camera,
                                    &mut last_camera_stuff,
                                    last_frame_time,
                                );
                            }

                            // End the UI frame. We could now handle the output and draw the UI with the backend.
                            let full_output = platform.end_frame(Some(&window));

                            let paint_jobs = platform.context().tessellate(full_output.shapes);

                            let screen_descriptor = ScreenDescriptor {
                                physical_width: window.inner_size().width,
                                physical_height: window.inner_size().height,
                                scale_factor: window.scale_factor() as f32,
                            };

                            egui_rpass.update_buffers(
                                device,
                                &graphics.queue,
                                &paint_jobs,
                                &screen_descriptor,
                            );

                            let tdelta: egui::TexturesDelta = full_output.textures_delta;

                            egui_rpass
                                .add_textures(device, &graphics.queue, &tdelta)
                                .expect("add texture ok");

                            egui_rpass
                                .execute(enc, view, &paint_jobs, &screen_descriptor, None)
                                .unwrap();

                            egui_rpass
                                .remove_textures(tdelta)
                                .expect("remove texture ok");
                        })),
                    )
                    .expect("Failed to render scene!");

                scene.update(&graphics);

                last_frame_time = start.elapsed();
            }
            _ => {}
        }
    });

    Ok(())
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
