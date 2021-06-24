use winit::event::{Event, VirtualKeyCode, ElementState,KeyboardInput, WindowEvent};
use winit::event_loop::{EventLoop, ControlFlow};

const WINDOW_TITLE: &str = "00 base code";
const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

struct App;

impl App {
    fn init_window(event_loop: &EventLoop<()>) -> winit::window::Window {
        winit::window::WindowBuilder::new()
            .with_title(WINDOW_TITLE)
            .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
            .build(event_loop)
            .expect("Failed to create window.")
    }

    pub fn main_loop(event_loop: EventLoop<()>) {
        event_loop.run(|event, _, control_flow| {
            match event {
                Event::WindowEvent { event, ..} => {
                    match event {
                        WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit;
                        },
                        WindowEvent::KeyboardInput { input, ..} => {
                            match input {
                                KeyboardInput { virtual_keycode, state, .. } => {
                                    match (virtual_keycode, state) {
                                        (Some(VirtualKeyCode::Escape), ElementState::Pressed) => {
                                            dbg!("按下Esc");
                                            *control_flow = ControlFlow::Exit;
                                        },
                                        _ => ()
                                    }
                                }
                            }
                        },
                        _ => ()
                    }
                },
                _ => (),
            }
        })
    }

    pub fn init_vulkan() {

    }

    pub fn clean_up() {
        
    }
}



fn main() {
    let event_loop = EventLoop::new();
    let _window = App::init_window(&event_loop);

    App::main_loop(event_loop);
}
