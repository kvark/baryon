pub struct Window {
    event_loop: winit::event_loop::EventLoop<()>,
    pub(crate) raw: winit::window::Window,
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
    Escape,
    Other,
}

pub enum Event {
    Resize { width: u32, height: u32 },
    Keyboard { key: Key, pressed: bool },
    Draw,
    Exit,
}

impl Window {
    pub fn new() -> WindowBuilder {
        WindowBuilder::default()
    }

    pub fn run(self, mut runner: impl 'static + FnMut(Event)) -> ! {
        use winit::{
            event::{
                ElementState, Event as WinEvent, KeyboardInput, VirtualKeyCode as Vkc, WindowEvent,
            },
            event_loop::ControlFlow,
        };

        self.event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                WinEvent::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    runner(Event::Resize {
                        width: size.width,
                        height: size.height,
                    });
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
                        } else if code == Vkc::Escape {
                            Key::Escape
                        } else {
                            log::debug!("Urecognized key {:?}", code);
                            Key::Other
                        },
                        pressed: state == ElementState::Pressed,
                    });
                }
                WinEvent::RedrawRequested(_) => {
                    runner(Event::Draw);
                }
                WinEvent::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                WinEvent::LoopDestroyed => {
                    runner(Event::Exit);
                }
                _ => {}
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
        let mut builder = winit::window::WindowBuilder::new();
        if let Some(title) = self.title {
            builder = builder.with_title(title);
        }
        if let Some(size) = self.size {
            builder = builder
                .with_inner_size(winit::dpi::Size::Physical((size.width, size.height).into()));
        }
        let raw = builder.build(&event_loop).unwrap();
        Window { raw, event_loop }
    }
}
