mod gpu;
mod input;
mod wasm;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    let event_loop = EventLoop::new();
    #[cfg(not(target_arch = "wasm32"))]
    let window = {
        let window = WindowBuilder::new()
            .with_title("Grainboy")
            .with_inner_size({
                let w = 256 * 4;
                let h = 144 * 4;
                winit::dpi::PhysicalSize::new(w, h)
            })
            .with_min_inner_size(winit::dpi::PhysicalSize::new(256, 144))
            .build(&event_loop)
            .unwrap();
        let monitor = window
            .current_monitor()
            .expect("Current monitor unavailable");
        let window_size = window.outer_size().cast::<i32>();
        let screen_size = monitor.size().cast::<i32>();
        let screen_pos = monitor.position().cast::<i32>();
        let x = screen_pos.x + (screen_size.width - window_size.width);
        let y = screen_pos.y;
        window.set_outer_position(winit::dpi::PhysicalPosition::new(x, y));
        window
    };
    let mut gpu = gpu::GPUContext::new(window).await;
    let mut renderer = gpu::Renderer::new(&gpu);
    let mut app: Option<wasm::App> = None;
    let mut tick = 0;
    let mut user_input = input::UserInput::new();
    event_loop.run(move |event, _, control_flow| match event {
        Event::MainEventsCleared => {
            if renderer.globals.tick > tick {
                if let Some(current_app) = &mut app {
                    current_app.clear_vertex_data();
                    current_app.update_input(user_input);
                    if let Err(err) = current_app.run() {
                        eprintln!("App error: {:?}", err);
                    }
                    current_app.read_vertex_data(|data| {
                        renderer.write_vertexes(&gpu, &bytemuck::cast_slice(data));
                    });
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if let Some(path) = &current_app.module_filepath {
                            match std::fs::File::open(&path) {
                                Err(err) => eprintln!("Error loading cart: {:?}", err),
                                Ok(file) => match file.metadata() {
                                    Err(err) => eprintln!("Error reading cart metadata: {:?}", err),
                                    Ok(metadata) => match metadata.created() {
                                        Err(err) => {
                                            eprintln!("Error cart metadata.created(): {:?}", err)
                                        }
                                        Ok(t) => {
                                            let created_at: chrono::DateTime<chrono::Utc> =
                                                t.into();
                                            if current_app.created_at.lt(&created_at) {
                                                match wasm::App::from_file(path) {
                                                    Err(err) => eprintln!(
                                                        "Error creating cart from file: {:?}",
                                                        err
                                                    ),
                                                    Ok(next_app) => {
                                                        let _ = app.insert(next_app);
                                                    }
                                                }
                                            }
                                        }
                                    },
                                },
                            }
                        }
                    }
                }
                user_input.main_events_cleared();
                tick = renderer.globals.tick;
            }
            match renderer.render(&gpu) {
                Ok(inst) => {
                    *control_flow = ControlFlow::WaitUntil(inst);
                }
                Err(wgpu::SurfaceError::Lost) => {
                    gpu.surface.configure(&gpu.device, &gpu.config);
                }
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(e) => eprintln!("{:?}", e),
            }
        }
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == gpu.window.id() => match event {
            WindowEvent::Resized(physical_size) => {
                gpu.config.width = physical_size.width;
                gpu.config.height = physical_size.height;
                gpu.surface.configure(&gpu.device, &gpu.config);
            }
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                gpu.config.width = new_inner_size.width;
                gpu.config.height = new_inner_size.height;
                gpu.surface.configure(&gpu.device, &gpu.config);
            }
            WindowEvent::DroppedFile(path) => {
                println!("DroppedFile {:#?}", path);
                if let Some(file) = path.to_str() {
                    match wasm::App::from_file(file) {
                        Ok(a) => {
                            let _ = app.insert(a);
                        }
                        Err(err) => eprintln!("Failed to load dropped file: {:?}", err),
                    }
                }
            }
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::KeyboardInput { input, .. } => match input {
                KeyboardInput {
                    state,
                    virtual_keycode,
                    ..
                } => {
                    use winit::event::{ElementState::*, VirtualKeyCode::*};
                    match (state, virtual_keycode) {
                        // Up
                        (Pressed, Some(Up) | Some(W)) => {
                            user_input.buttons.up = user_input.buttons.up.next(*state);
                        }
                        (Released, Some(Up) | Some(W)) => {
                            user_input.buttons.up = user_input.buttons.up.next(*state);
                        }
                        // Down
                        (Pressed, Some(Down) | Some(S)) => {
                            user_input.buttons.down = user_input.buttons.down.next(*state);
                        }
                        (Released, Some(Down) | Some(S)) => {
                            user_input.buttons.down = user_input.buttons.down.next(*state);
                        }
                        // Left
                        (Pressed, Some(Left) | Some(A)) => {
                            user_input.buttons.left = user_input.buttons.left.next(*state);
                        }
                        (Released, Some(Left) | Some(A)) => {
                            user_input.buttons.left = user_input.buttons.left.next(*state);
                        }
                        // Right
                        (Pressed, Some(Right) | Some(D)) => {
                            user_input.buttons.right = user_input.buttons.right.next(*state);
                        }
                        (Released, Some(Right) | Some(D)) => {
                            user_input.buttons.right = user_input.buttons.right.next(*state);
                        }
                        // A
                        (Pressed, Some(Z)) => {
                            user_input.buttons.a = user_input.buttons.a.next(*state);
                        }
                        (Released, Some(Z)) => {
                            user_input.buttons.a = user_input.buttons.a.next(*state);
                        }
                        // B
                        (Pressed, Some(X)) => {
                            user_input.buttons.b = user_input.buttons.b.next(*state);
                        }
                        (Released, Some(X)) => {
                            user_input.buttons.b = user_input.buttons.b.next(*state);
                        }
                        // X
                        (Pressed, Some(C)) => {
                            user_input.buttons.x = user_input.buttons.x.next(*state);
                        }
                        (Released, Some(C)) => {
                            user_input.buttons.x = user_input.buttons.x.next(*state);
                        }
                        // Y
                        (Pressed, Some(V)) => {
                            user_input.buttons.y = user_input.buttons.y.next(*state);
                        }
                        (Released, Some(V)) => {
                            user_input.buttons.y = user_input.buttons.y.next(*state);
                        }
                        // START
                        (Pressed, Some(Space)) => {
                            user_input.buttons.start = user_input.buttons.start.next(*state);
                        }
                        (Released, Some(Space)) => {
                            user_input.buttons.start = user_input.buttons.start.next(*state);
                        }
                        // SELECT
                        (Pressed, Some(Return)) => {
                            user_input.buttons.select = user_input.buttons.select.next(*state);
                        }
                        (Released, Some(Return)) => {
                            user_input.buttons.select = user_input.buttons.select.next(*state);
                        }
                        (Pressed, Some(Escape)) => {
                            *control_flow = ControlFlow::Exit;
                        }
                        _ => (),
                    }
                }
            },
            WindowEvent::MouseInput { button, state, .. } => {
                use MouseButton::*;
                match button {
                    Left => {
                        user_input.mouse.left = user_input.mouse.left.next(state.clone());
                        println!("MouseInput {:?}", button);
                    }
                    Right => {
                        user_input.mouse.right = user_input.mouse.right.next(state.clone());
                        println!("MouseInput {:?}", button);
                    }
                    _ => (),
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let size = position.to_logical::<i32>(gpu.window.scale_factor());
                let x = size.x / 4;
                let y = size.y / 4;
                user_input.cursor = [x, y];
            }
            WindowEvent::MouseWheel { delta, .. } => {
                use MouseScrollDelta::*;
                user_input.wheel = match delta {
                    PixelDelta(delta) => {
                        let delta = delta.to_logical::<i32>(gpu.window.scale_factor());
                        let x = delta.x / 4;
                        let y = delta.y / 4;
                        [x, y]
                    }
                    LineDelta(x, y) => {
                        // We'll just call it 8 pixels per line I guess 🤷🏽‍♂️
                        let x = *x as i32 * 8;
                        let y = -y as i32 * 8;
                        [x, y]
                    }
                };
            }
            _ => {}
        },
        _ => {}
    });
}
