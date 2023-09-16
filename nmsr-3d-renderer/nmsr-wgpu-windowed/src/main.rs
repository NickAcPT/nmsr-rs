use std::time::{Duration, Instant};

use egui::emath::Numeric;
use egui::{Context, FontDefinitions};
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};

use nmsr_player_parts::parts::part::Part;
use nmsr_rendering::high_level::pipeline::scene::{self, Scene, SunInformation, Size};
use nmsr_rendering::high_level::pipeline::{
    GraphicsContext, GraphicsContextDescriptor, SceneContext, SceneContextWrapper,
};
use nmsr_rendering::low_level::Vec3;
use nmsr_rendering::low_level::primitives::part_primitive::PartPrimitive;
use strum::IntoEnumIterator;

use wgpu::{Backends, Instance, Features};
use winit::event;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;

use nmsr_player_parts::parts::provider::PlayerPartProviderContext;
use nmsr_player_parts::model::PlayerModel;
use nmsr_player_parts::types::PlayerBodyPartType;
use nmsr_rendering::high_level::camera::{
    Camera, CameraPositionParameters, CameraRotation, ProjectionParameters,
};
use winit::platform::run_return::EventLoopExtRunReturn;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    //mem::forget(unsafe {
    //    libloading::Library::new("D:\\IDEs\\CLionProjects\\nmsr-wgpu\\vulkan-1.dll").unwrap()
    //});

    let mut renderdoc =
        renderdoc::RenderDoc::<renderdoc::V140>::new().expect("Failed to initialize RenderDoc");

    //renderdoc
    //    .launch_replay_ui(true, None)
    //    .expect("Failed to launch RenderDoc replay UI");

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
        features: Features::empty(),
        blend_state: None,
        sample_count: None,
        use_smaa: None,
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

    let sun = SunInformation::new([0.0, -1.0, 5.0].into(), 1.0, 0.35);
    
    let camera = Camera::new_absolute(
        Vec3::new(0.0, 30.0, -20.0),
        CameraRotation {
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
        },
        ProjectionParameters::Perspective { fov: 110f32 },
        Some(Size {
            width: config.width,
            height: config.height,
        }),
    );

    let mut ctx = PlayerPartProviderContext {
        model: PlayerModel::Alex,
        has_layers: true,
        has_hat_layer: true,
        has_cape: true,
        arm_rotation: 0.0,
        shadow_y_pos: Some(0.0),
        shadow_is_square: false,
        armor_slots: None,
        #[cfg(feature = "ears")] ears_features: None
    };

    let mut scene = build_scene(&graphics, config, &mut ctx, camera, sun);

    println!("surface_view_format: {:?}", surface_view_format);
    println!("MSAA samples: {:?}", &graphics.multisampling_strategy);

    println!("Entering render loop...");
    let start_time = Instant::now();
    let mut last_frame_time = Duration::ZERO;

    let mut last_camera_stuff: Option<(CameraPositionParameters, CameraRotation, ProjectionParameters)> = None;

    let mut egui_rpass = RenderPass::new(device, *surface_view_format, 1);

    let mut platform = Platform::new(PlatformDescriptor {
        physical_width: config.width,
        physical_height: config.height,
        scale_factor: window.scale_factor(),
        font_definitions: FontDefinitions::default(),
        style: Default::default(),
    });
    
    let mut last_computed_parts = scene.parts().to_vec();

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
                    device.poll(wgpu::MaintainBase::Poll);
                    surface.configure(device, &new_config);
                    camera.set_size(Some(Size {
                        width: new_config.width,
                        height: new_config.height,
                    }));

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

                let mut needs_rebuild = false;
                let mut needs_skin_rebuild = false;

                scene
                    .render_with_extra(
                        &graphics,
                        Some(Box::new(|view, enc, camera, sun| {
                            platform.update_time(start_time.elapsed().as_secs_f64());

                            platform.begin_frame();

                            {
                                debug_ui(
                                    &platform.context(),
                                    camera,
                                    sun,
                                    &mut last_camera_stuff,
                                    last_frame_time,
                                    &mut ctx,
                                    &mut needs_rebuild,
                                    &mut needs_skin_rebuild,
                                    &last_computed_parts
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

                if needs_rebuild {
                    last_computed_parts = scene.rebuild_parts(&ctx, PlayerBodyPartType::iter().collect()).to_vec();
                }

                if needs_skin_rebuild {
                    let skin_bytes: Vec<u8> = match ctx.model {
                        PlayerModel::Steve => include_bytes!("aaaa.png").to_vec(),
                        PlayerModel::Alex => include_bytes!("6985d6a236558d495f25d57f15fa3851f2d6af5493bc408b8f627a7232a7fb (1).png").to_vec(),
                    };
                    let skin_image = image::load_from_memory(&skin_bytes).unwrap();
                    let mut skin_rgba = skin_image.to_rgba8();

                    ears_rs::utils::strip_alpha(&mut skin_rgba);

                    // Upload skin and cape
                    scene.set_texture(
                        &graphics,
                        nmsr_player_parts::types::PlayerPartTextureType::Skin,
                        &skin_rgba,
                    );
                    
                }

                last_frame_time = start.elapsed();

                *control_flow = winit::event_loop::ControlFlow::Poll;
            }
            _ => {}
        }
    });

    Ok(())
}

fn build_scene(
    graphics: &GraphicsContext,
    config: &wgpu::SurfaceConfiguration,
    ctx: &mut PlayerPartProviderContext,
    camera: Camera,
    sun: SunInformation
) -> Scene<SceneContextWrapper> {
    let skin_bytes =
        include_bytes!("ears_v0_sample_ear_out_front_claws_horn_tail_back_3_snout_4x3x4-0,2_wings_symmetric_dual_normal.png");
    let skin_image = image::load_from_memory(skin_bytes).unwrap();
    let mut skin_rgba = skin_image.to_rgba8();
    
    #[cfg(feature = "ears")]
    {
        ctx.ears_features = ears_rs::parser::EarsParser::parse(&skin_rgba).unwrap();
    }
    
    let mut scene = Scene::new(
        graphics,
        SceneContext::new(graphics).into(),
        camera,
        sun,
        scene::Size {
            width: config.width,
            height: config.height,
        },
        ctx,
        &PlayerBodyPartType::iter().collect::<Vec<_>>(),
    );

    // Create pipeline layout
    ears_rs::utils::strip_alpha(&mut skin_rgba);

    // Upload skin and cape
    scene.set_texture(
        graphics,
        nmsr_player_parts::types::PlayerPartTextureType::Skin,
        &skin_rgba,
    );

    let cape_rgba = image::load_from_memory(include_bytes!("download (17).png"))
        .unwrap()
        .to_rgba8();

    scene.set_texture(
        graphics,
        nmsr_player_parts::types::PlayerPartTextureType::Cape,
        &cape_rgba,
    );

    scene
}

fn debug_ui(
    ctx: &Context,
    camera: &mut Camera,
    sun: &mut SunInformation,
    last_camera_stuff: &mut Option<(CameraPositionParameters, CameraRotation, ProjectionParameters)>,
    last_frame_time: Duration,
    part_ctx: &mut PlayerPartProviderContext,
    needs_rebuild: &mut bool,
    needs_skin_rebuild: &mut bool,
    last_computed_parts: &Vec<Part>
) {
    egui::Window::new("Camera").vscroll(true).show(ctx, |ui| {
        ui.label(format!("Last Frame time: {:?}", last_frame_time));
        
        ui.separator();
        
        ui.label("Presets");

        ui.horizontal(|ui| {
            if ui.button("Visage").clicked() {
                visage(camera, last_camera_stuff);
            }
            if ui.button("Visage (Orbital)").clicked() {
                visage_orbital(camera, last_camera_stuff);
            }

            if last_camera_stuff.is_some() && ui.button("Last").clicked() {
                let current = (camera.get_position_parameters(), camera.get_rotation(), camera.get_projection());

                let (position, rotation, projection) = last_camera_stuff.unwrap();
                camera.set_position_parameters(position);
                camera.set_rotation(rotation);
                camera.set_projection(projection);

                last_camera_stuff.replace(current);
            }
        });

        ui.horizontal(|ui| {
            if ui.button("NMSR (Full Body)").clicked() {

                camera.set_position_parameters(CameraPositionParameters::Absolute(Vec3::new(
                    21.47,
                    27.31,
                    -46.48,
                )));

                camera.set_rotation(CameraRotation {
                    yaw: 24.28,
                    pitch: 11.83,
                    roll: 0.0,
                });

                camera.set_projection(ProjectionParameters::Perspective { fov: 37.6772850524784 });
            }
            
            if ui.button("NMSR (Head)").clicked() {
                camera.set_position_parameters(CameraPositionParameters::Absolute(Vec3::new(
                    10.16,
                    33.5,
                    -22.40,
                )));
                
                camera.set_rotation(CameraRotation {
                    yaw: 25.26,
                    pitch: 14.95,
                    roll: 0.0,
                });
                
                camera.set_projection(ProjectionParameters::Perspective { fov: 27.1 });
                
            }
            
        });

        ui.horizontal(|ui| {
            if ui.button("NMSR (Full Body) (Orbital)").clicked() {

                camera.set_position_parameters(CameraPositionParameters::Orbital{
                    distance: 53.0,
                    look_at: [0.2, 16.5, 0.5].into()
                });

                camera.set_rotation(CameraRotation {
                    yaw: 24.28,
                    pitch: 11.83,
                    roll: 0.0,
                });

                camera.set_projection(ProjectionParameters::Perspective { fov: 37.5 });
            }
            
            if ui.button("NMSR (Head) (Orbital)").clicked() {
                camera.set_position_parameters(CameraPositionParameters::Absolute(Vec3::new(
                    10.15629061247147,
                    33.73435909438063,
                    -22.408843189323385,
                )));
                
                camera.set_rotation(CameraRotation {
                    yaw: 25.264535906746477,
                    pitch: 14.953989778518544,
                    roll: 0.0,
                });
                
                camera.set_projection(ProjectionParameters::Perspective { fov: 23.444515728494967 });
                
            }
            
        });
        
        ui.separator();
        
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
            ui.horizontal(|ui| {
                    
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
            });
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

        ui.horizontal(|ui| {
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
            ui.label("Roll");
            ui.add(drag_value(
                camera,
                Camera::get_roll,
                Camera::set_roll,
                Some(-180.0f32),
                Some(180.0f32),
            ));
            
        });

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
        
        if ui.button("Center isometric").clicked() {
            
            // First thing we do is get our scene parts
            let parts_pos: Vec<_> = last_computed_parts.iter()
                .map(scene::primitive_convert)
                .flat_map(|p| p.get_vertices())
                .map(|v| v.position)
                .collect();
            
            let first = parts_pos.first().unwrap();
            
            let min = parts_pos.iter().fold(*first, |acc, v| acc.min(*v));
            let max = parts_pos.iter().fold(*first, |acc, v| acc.max(*v));
            
            let center = (min + max) / 2.0;
            
            let center = center * Into::<Vec3>::into([-1.0, 1.0, -1.0]);
            
            camera.set_position_parameters(CameraPositionParameters::Absolute(center));
            
            // Isometric camera rotation
            camera.set_rotation(CameraRotation {
                yaw: 45.0,
                pitch: 30.0f32.to_radians().asin().to_degrees(),
                roll: 0.0,
            });
            
            let diameter = (max - min).length();
            let aspect = camera.get_aspect_ratio();
            
            println!("max - min: {}", max - min);
            println!("aspect: {}", aspect);
            println!("diameter / 2.0 / aspect: {}", diameter / 2.0 / aspect);
            println!("diameter / 2.0 * aspect: {}", diameter / 2.0 * aspect);
            println!("diameter / 2.0: {}", diameter / 2.0);
            
            let scale = if aspect > 1.0 {
                diameter / 2.0
            } else {
                diameter / 2.0 / aspect
            };
            
            println!("Diameter: {diameter:?}, Min: {min:?}, Max: {max:?}, Center: {center:?}");
            
            camera.set_projection(ProjectionParameters::Orthographic { aspect: scale })
        }
    });

    egui::Window::new("Part Context")
        .vscroll(true)
        .show(ctx, |ui| {
            //part_ctx.has_cape
            //part_ctx.has_layers
            //part_ctx.model
            //part_ctx.arm_rotation

            ui.label("Model");
            ui.horizontal(|ui| {
                *needs_skin_rebuild |= ui
                    .radio_value(&mut part_ctx.model, PlayerModel::Steve, "Steve")
                    .changed();
                *needs_skin_rebuild |= ui
                    .radio_value(&mut part_ctx.model, PlayerModel::Alex, "Alex")
                    .changed();

                *needs_rebuild |= *needs_skin_rebuild;
            });

            ui.label("Arm Rotation");
            *needs_rebuild |= ui
                .add(drag_value(
                    part_ctx,
                    |ctx| ctx.arm_rotation,
                    |ctx, v| ctx.arm_rotation = v,
                    Some(0.0f32),
                    Some(360.0f32),
                ))
                .changed();

            *needs_rebuild |= ui.checkbox(&mut part_ctx.has_layers, "Has Layers").changed();

            *needs_rebuild |= ui.checkbox(&mut part_ctx.has_hat_layer, "Has Hat").changed();
            
            *needs_rebuild |= ui.checkbox(&mut part_ctx.has_cape, "Has Cape").changed();
        });
        
    egui::Window::new("Sun")
    .vscroll(true)
    .show(ctx, |ui| {
        // sun direction
        // sun intensity
        
        ui.label("Direction");
        ui.horizontal(|ui| {
            ui.label("X");
            *needs_rebuild |= ui.add(drag_value(
                sun,
                |sun| sun.direction.x,
                |sun, v| sun.direction.x = v,
                None,
                None,
            )).changed();
            ui.label("Y");
            *needs_rebuild |= ui.add(drag_value(
                sun,
                |sun| sun.direction.y,
                |sun, v| sun.direction.y = v,
                None,
                None,
            )).changed();
            ui.label("Z");
            *needs_rebuild |= ui.add(drag_value(
                sun,
                |sun| sun.direction.z,
                |sun, v| sun.direction.z = v,
                None,
                None,
            )).changed();
        });
        
        ui.label("Intensity");
        *needs_rebuild |= ui.add(drag_value(
            sun,
            |sun| sun.intensity,
            |sun, v| sun.intensity = v,
            None,
            None,
        )).changed();
        
        ui.label("Ambient");
        *needs_rebuild |= ui.add(drag_value(
            sun,
            |sun| sun.ambient,
            |sun, v| sun.ambient = v,
            None,
            None,
        )).changed();
        
    });
}

fn visage_orbital(
    camera: &mut Camera,
    last_camera_stuff: &mut Option<(CameraPositionParameters, CameraRotation, ProjectionParameters)>,
) {
    last_camera_stuff.replace((camera.get_position_parameters(), camera.get_rotation(), camera.get_projection()));

    camera.set_position_parameters(CameraPositionParameters::Orbital {
        look_at: [0.0, 16.65, 0.0].into(),
        distance: 44.1,
    });

    camera.set_projection(ProjectionParameters::Perspective { fov: 45.0 });

    camera.set_rotation(CameraRotation {
        yaw: 20.0,
        pitch: 10.0,
        roll: 0.0,
    })
}

fn visage(
    camera: &mut Camera,
    last_camera_stuff: &mut Option<(CameraPositionParameters, CameraRotation, ProjectionParameters)>,
) {
    last_camera_stuff.replace((camera.get_position_parameters(), camera.get_rotation(), camera.get_projection()));

    camera.set_position_parameters(CameraPositionParameters::Absolute(Vec3::new(
        14.85, 24.3, -40.85,
    )));

    camera.set_projection(ProjectionParameters::Perspective { fov: 45.0 });

    camera.set_rotation(CameraRotation {
        yaw: 20.0,
        pitch: 10.0,
        roll: 0.0,
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
