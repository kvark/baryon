use std::time;

//TODO: a mechanism like this should be a part of the engine
struct Animator {
    map: baryon::asset::SpriteMap,
    cell_counts: mint::Vector2<usize>,
    duration: time::Duration,
    sprite: baryon::EntityRef,
    current: mint::Point2<usize>,
    moment: time::Instant,
}

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum State {
    Idle = 0,
    MoveRight = 9,
    MoveLeft = 8,
    Kick = 4,
    Jump = 10,
    Lie = 12,
}
impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}

impl Animator {
    fn update_uv(&mut self, scene: &mut baryon::Scene) {
        let uv_range = self.map.at(self.current);
        scene
            .world
            .get_mut::<baryon::Sprite>(self.sprite)
            .unwrap()
            .uv = Some(uv_range);
    }

    fn switch(&mut self, state: State, scene: &mut baryon::Scene) {
        self.moment = time::Instant::now();
        self.current.x = 0;
        self.current.y = state as usize;
        self.update_uv(scene);
    }

    fn tick(&mut self, scene: &mut baryon::Scene) {
        if self.moment.elapsed() < self.duration {
            return;
        }

        self.current.x += 1;
        self.moment = time::Instant::now();
        if self.current.x == self.cell_counts.x {
            self.current.x = 0;
            self.current.y = State::Idle as usize;
            // don't update the scene here, so that
            // input can have a chance to transition
            // to something other than `Idle`.
        } else {
            self.update_uv(scene);
        }
    }
}

fn main() {
    use baryon::window::{Event, Key, Window};

    let window = Window::new().title("Sprite").build();
    let mut context = pollster::block_on(baryon::Context::init().build(&window));
    let mut scene = baryon::Scene::new();
    let camera = baryon::Camera {
        projection: baryon::Projection::Orthographic {
            // the sprite configuration is not centered
            center: [0.0, -10.0].into(),
            extent_y: 40.0,
        },
        ..Default::default()
    };
    let mut pass = baryon::pass::Flat::new(&context);

    let image = context.load_image(format!(
        "{}/examples/assets/pickachu.png",
        env!("CARGO_MANIFEST_DIR")
    ));
    let sprite = scene.add_sprite(image).build();

    let mut anim = Animator {
        map: baryon::asset::SpriteMap {
            origin: mint::Point2 { x: 0, y: 0 },
            cell_size: mint::Vector2 { x: 96, y: 96 },
        },
        cell_counts: mint::Vector2 { x: 5, y: 13 },
        duration: time::Duration::from_secs_f64(0.1),
        current: mint::Point2 { x: 0, y: 0 },
        moment: time::Instant::now(),
        sprite,
    };
    anim.switch(State::Idle, &mut scene);

    window.run(move |event| match event {
        Event::Resize { width, height } => {
            context.resize(width, height);
        }
        Event::Keyboard { key, pressed: true } => {
            let new_state = match key {
                Key::Up => Some(State::Jump),
                Key::Down => Some(State::Lie),
                Key::Space => Some(State::Kick),
                Key::Left => Some(State::MoveLeft),
                Key::Right => Some(State::MoveRight),
                _ => None,
            };
            if let Some(state) = new_state {
                if anim.current.y != state as usize || state == State::Kick {
                    anim.switch(state, &mut scene);
                }
            }
        }
        Event::Draw => {
            anim.tick(&mut scene);
            context.present(&mut pass, &scene, &camera);
        }
        _ => {}
    })
}
