use std::time;

//TODO: a mechanism like this should be a part of the engine
struct Animator {
    cell_size: mint::Vector2<i16>,
    cell_counts: mint::Vector2<i16>,
    duration: time::Duration,
    repeat: bool,
    sprite: baryon::EntityRef,
    current: mint::Point2<i16>,
    moment: time::Instant,
}

impl Animator {
    fn update_uv(&mut self, scene: &mut baryon::Scene) {
        let begin = mint::Point2 {
            x: self.current.x * self.cell_size.x,
            y: self.current.y * self.cell_size.y,
        };
        let end = mint::Point2 {
            x: begin.x + self.cell_size.x,
            y: begin.y + self.cell_size.y,
        };
        scene
            .world
            .get_mut::<baryon::Sprite>(self.sprite)
            .unwrap()
            .uv = Some(begin..end);
    }

    fn switch(&mut self, change_row: i16, scene: &mut baryon::Scene) {
        self.moment = time::Instant::now();
        self.current.x = 0;
        self.current.y = (self.current.y + change_row).rem_euclid(self.cell_counts.y);
        self.update_uv(scene);
    }

    fn tick(&mut self, scene: &mut baryon::Scene) {
        if self.moment.elapsed() >= self.duration
            && (self.repeat || self.current.x < self.cell_counts.x)
        {
            self.moment = time::Instant::now();
            self.current.x += 1;
            if self.current.x < self.cell_counts.x {
                self.update_uv(scene);
            } else if self.repeat {
                self.current.x = 0;
                self.update_uv(scene);
            }
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
    let mut pass = baryon::pass::Flat::new(context.surface_format().unwrap(), &context);

    let image = context.load_image(format!(
        "{}/examples/assets/pickachu.png",
        env!("CARGO_MANIFEST_DIR")
    ));
    let sprite = scene.add_sprite(image).build();

    let mut anim = Animator {
        cell_size: mint::Vector2 { x: 96, y: 96 },
        cell_counts: mint::Vector2 { x: 5, y: 13 },
        duration: time::Duration::from_secs_f64(0.1),
        repeat: true,
        current: mint::Point2 { x: 0, y: 0 },
        moment: time::Instant::now(),
        sprite,
    };
    anim.update_uv(&mut scene);

    window.run(move |event| match event {
        Event::Resize { width, height } => {
            context.resize(width, height);
        }
        Event::Keyboard {
            key: Key::Up,
            pressed: true,
        } => {
            anim.switch(1, &mut scene);
        }
        Event::Keyboard {
            key: Key::Down,
            pressed: true,
        } => {
            anim.switch(-1, &mut scene);
        }
        Event::Draw => {
            anim.tick(&mut scene);
            context.present(&mut pass, &scene, &camera);
        }
        _ => {}
    })
}
