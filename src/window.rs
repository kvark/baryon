use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

const TARGET_FRAME_TIME: f64 = 1.0 / 120.0;

pub struct Window {
    event_loop: winit::event_loop::EventLoop<()>,
    raw: winit::window::Window,
}

unsafe impl HasRawWindowHandle for Window {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.raw.raw_window_handle()
    }
}

impl bc::HasWindow for Window {
    fn size(&self) -> mint::Vector2<u32> {
        let size = self.raw.inner_size();
        mint::Vector2 {
            x: size.width,
            y: size.height,
        }
    }
}

#[derive(Default)]
pub struct WindowBuilder {
    title: Option<String>,
    size: Option<wgpu::Extent3d>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Key {
    Digit(u8),
    Letter(char),
    Function(u8),
    Up,
    Down,
    Left,
    Right,
    Space,
    Escape,
    Other,
}

pub enum Event {
    Resize { width: u32, height: u32 },
    Keyboard { key: Key, pressed: bool },
    Pointer { position: mint::Vector2<f32> },
    Scroll { delta: mint::Vector2<f32> },
    Draw,
    Exit,
}

impl Window {
    pub fn new() -> WindowBuilder {
        WindowBuilder::default()
    }

    pub fn run(self, mut runner: impl 'static + FnMut(Event)) -> ! {
        use std::time;
        use winit::{
            event::{
                ElementState, Event as WinEvent, KeyboardInput, MouseScrollDelta, VirtualKeyCode as Vkc, WindowEvent,
            },
            event_loop::ControlFlow,
        };

        let mut last_update_inst = time::Instant::now();
        let Self {
            event_loop,
            raw: window,
        } = self;

        event_loop.run(move |event, _, control_flow| {
            *control_flow = match event {
                WinEvent::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    runner(Event::Resize {
                        width: size.width,
                        height: size.height,
                    });
                    ControlFlow::Poll
                }
                WinEvent::WindowEvent {
                    event:
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state,
                                    virtual_keycode: Some(code),
                                    ..
                                },
                            ..
                        },
                    ..
                } => {
                    runner(Event::Keyboard {
                        key: if code >= Vkc::Key1 && code <= Vkc::Key0 {
                            Key::Digit(code as u8 - Vkc::Key1 as u8)
                        } else if code >= Vkc::A && code <= Vkc::Z {
                            Key::Letter((code as u8 - Vkc::A as u8) as char)
                        } else if code >= Vkc::F1 && code <= Vkc::F12 {
                            Key::Function(code as u8 - Vkc::F1 as u8)
                        } else {
                            match code {
                                Vkc::Left => Key::Left,
                                Vkc::Right => Key::Right,
                                Vkc::Up => Key::Up,
                                Vkc::Down => Key::Down,
                                Vkc::Space => Key::Space,
                                Vkc::Escape => Key::Escape,
                                _ => {
                                    log::debug!("Urecognized key {:?}", code);
                                    Key::Other
                                }
                            }
                        },
                        pressed: state == ElementState::Pressed,
                    });
                    ControlFlow::Poll
                }
                WinEvent::WindowEvent {
                    event: WindowEvent::CursorMoved { position, .. },
                    ..
                } => {
                    runner(Event::Pointer { position: mint::Vector2 { x: position.x as f32, y: position.y as f32 } });
                    ControlFlow::Poll
                }
                WinEvent::WindowEvent {
                    event: WindowEvent::MouseWheel { delta, .. },
                    ..
                } => {
                    match delta {
                      MouseScrollDelta::LineDelta(x, y)=>{
                          runner(Event::Scroll { delta: mint::Vector2 { x, y } });
                      },
                      MouseScrollDelta::PixelDelta(position)=>{
                          runner(Event::Scroll { delta: mint::Vector2 { x: position.x as f32, y: position.y as f32 } });
                      }
                    }
                    ControlFlow::Poll
                }
                WinEvent::RedrawRequested(_) => {
                    runner(Event::Draw);
                    ControlFlow::Poll
                }
                WinEvent::RedrawEventsCleared => {
                    let target_frametime = time::Duration::from_secs_f64(TARGET_FRAME_TIME);
                    let now = time::Instant::now();
                    match target_frametime.checked_sub(last_update_inst.elapsed()) {
                        Some(wait_time) => ControlFlow::WaitUntil(now + wait_time),
                        None => {
                            window.request_redraw();
                            last_update_inst = now;
                            ControlFlow::Poll
                        }
                    }
                }
                WinEvent::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => ControlFlow::Exit,
                WinEvent::LoopDestroyed => {
                    runner(Event::Exit);
                    ControlFlow::Exit
                }
                _ => ControlFlow::Poll,
            }
        })
    }
}

impl WindowBuilder {
    pub fn title(self, title: &str) -> Self {
        Self {
            title: Some(title.to_string()),
            ..self
        }
    }

    pub fn size(self, width: u32, height: u32) -> Self {
        Self {
            size: Some(wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            }),
            ..self
        }
    }

    pub fn build(self) -> Window {
        let event_loop = winit::event_loop::EventLoop::new();
        let mut builder = winit::window::WindowBuilder::new()
            .with_min_inner_size(winit::dpi::Size::Logical((64, 64).into()));
        if let Some(title) = self.title {
            builder = builder.with_title(title);
        }
        if let Some(size) = self.size {
            builder = builder
                .with_inner_size(winit::dpi::Size::Logical((size.width, size.height).into()));
        }
        let raw = builder.build(&event_loop).unwrap();
        Window { raw, event_loop }
    }
}
