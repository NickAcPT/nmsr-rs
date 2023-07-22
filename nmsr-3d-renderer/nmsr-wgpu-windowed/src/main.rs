use wgpu::RequestAdapterOptions;
use winit::event;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;

#[tokio::main]
async fn main() {
    let event_loop = EventLoop::new();
    let mut builder = winit::window::WindowBuilder::new();
    let window = builder.build(&event_loop).unwrap();

    let backends = wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);
    let dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends,
        dx12_shader_compiler,
    });

    let (size, surface) = unsafe {
        let size = window.inner_size();

        let surface = instance.create_surface(&window).unwrap();

        (size, surface)
    };

    let adapter = instance.request_adapter(&RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
    }).await.expect("Failed to find an appropiate adapter");


    let adapter_info = adapter.get_info();
    println!("Using {} ({:?})", adapter_info.name, adapter_info.backend);

    let (device, queue) = adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            features: wgpu::Features::empty(),
            limits: wgpu::Limits::default(),
        },
        None,
    ).await.expect("Unable to find a suitable GPU adapter!");

    let mut config = surface
        .get_default_config(&adapter, size.width, size.height)
        .expect("Surface isn't supported by the adapter.");
    let surface_view_format = config.format.add_srgb_suffix();
    config.view_formats.push(surface_view_format);
    surface.configure(&device, &config);

    println!("Entering render loop...");
    event_loop.run(move |event, _, control_flow| {
        match event {
            event::Event::RedrawEventsCleared => {
                window.request_redraw();
            },
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
                        size,
                        max_dimension
                    );
                } else {
                    println!("Resizing to {:?}", size);
                    config.width = size.width.max(1);
                    config.height = size.height.max(1);
                    surface.configure(&device, &config);
                }
            },
            event::Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = winit::event_loop::ControlFlow::Exit;
            },
            event::Event::RedrawRequested(_) => {
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

                frame.present();
            },
            _ => {
                println!("{:?}", event);
            }
        }
    });
}
