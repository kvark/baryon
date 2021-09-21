use std::time;

fn create_prototype(builder: baryon::MeshBuilder) -> baryon::Prototype {
    fn vertex(x: i8, y: i8, z: i8) -> baryon::Position {
        baryon::Position([x as f32, y as f32, z as f32])
    }

    //TODO: can squeeze this by a factor of 3
    let vertex_data = [
        // top (0, 0, 1)
        vertex(-1, -1, 1),
        vertex(1, -1, 1),
        vertex(1, 1, 1),
        vertex(-1, 1, 1),
        // bottom (0, 0, -1)
        vertex(-1, 1, -1),
        vertex(1, 1, -1),
        vertex(1, -1, -1),
        vertex(-1, -1, -1),
        // right (1, 0, 0)
        vertex(1, -1, -1),
        vertex(1, 1, -1),
        vertex(1, 1, 1),
        vertex(1, -1, 1),
        // left (-1, 0, 0)
        vertex(-1, -1, 1),
        vertex(-1, 1, 1),
        vertex(-1, 1, -1),
        vertex(-1, -1, -1),
        // front (0, 1, 0)
        vertex(1, 1, -1),
        vertex(-1, 1, -1),
        vertex(-1, 1, 1),
        vertex(1, 1, 1),
        // back (0, -1, 0)
        vertex(1, -1, 1),
        vertex(-1, -1, 1),
        vertex(-1, -1, -1),
        vertex(1, -1, -1),
    ];

    let index_data = [
        0u16, 1, 2, 2, 3, 0, // top
        4, 5, 6, 6, 7, 4, // bottom
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // front
        20, 21, 22, 22, 23, 20, // back
    ];

    builder
        .vertex(&vertex_data)
        .index(&index_data)
        .radius(1.0)
        .build()
}

struct Cube {
    node: baryon::NodeRef,
    level: u8,
}

const SCALE_ROOT: f32 = 2.0;
const SCALE_LEVEL: f32 = 0.4;

struct Level {
    color: baryon::Color,
    speed: f32,
}

fn fill_scene(
    levels: &[Level],
    scene: &mut baryon::Scene,
    prototype: &baryon::Prototype,
) -> Vec<Cube> {
    let root_node = scene.add_node().scale(SCALE_ROOT).build();
    scene
        .add_entity(prototype)
        .parent(root_node)
        .component(levels[0].color)
        .build();
    let mut list = vec![Cube {
        node: root_node,
        level: 0,
    }];

    struct Stack {
        parent: baryon::NodeRef,
        level: u8,
    }
    let mut stack = vec![Stack {
        parent: root_node,
        level: 1,
    }];

    let children = [
        mint::Vector3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
        mint::Vector3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
        mint::Vector3 {
            x: -1.0,
            y: 0.0,
            z: 0.0,
        },
        mint::Vector3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
        mint::Vector3 {
            x: 0.0,
            y: -1.0,
            z: 0.0,
        },
    ];

    while let Some(next) = stack.pop() {
        let level = match levels.get(next.level as usize) {
            Some(level) => level,
            None => continue,
        };
        for &child in children.iter() {
            let node = scene
                .add_node()
                .position(mint::Vector3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0 + SCALE_LEVEL,
                })
                .scale(SCALE_LEVEL)
                .parent(next.parent)
                .build();
            scene[node].post_rotate(child, 90.0);

            scene
                .add_entity(prototype)
                .parent(node)
                .component(level.color)
                .build();
            list.push(Cube {
                node,
                level: next.level,
            });

            stack.push(Stack {
                parent: node,
                level: next.level + 1,
            });
        }
    }

    list
}

const LEVELS: &[Level] = &[
    Level {
        color: baryon::Color(0xFFFFFF80),
        speed: 20.0,
    },
    Level {
        color: baryon::Color(0xFF8080FF),
        speed: -30.0,
    },
    Level {
        color: baryon::Color(0xFF80FF80),
        speed: 40.0,
    },
    Level {
        color: baryon::Color(0xFFFF8080),
        speed: -60.0,
    },
    Level {
        color: baryon::Color(0xFF80FFFF),
        speed: 80.0,
    },
    Level {
        color: baryon::Color(0xFFFF80FF),
        speed: -100.0,
    },
];

fn main() {
    use baryon::window::{Event, Window};

    env_logger::init();
    let window = Window::new().title("Cubeception").build();
    let mut context = pollster::block_on(baryon::Context::init().build(&window));
    let mut scene = baryon::Scene::new();

    let camera = baryon::Camera {
        projection: baryon::Projection::Perspective { fov_y: 45.0 },
        depth: 1.0..10.0,
        node: scene
            .add_node()
            .position([1.8f32, -8.0, 3.0].into())
            .look_at([0f32; 3].into(), [0f32, 0.0, 1.0].into())
            .build(),
        background: baryon::Color(0xFF203040),
    };

    let prototype = create_prototype(context.add_mesh());
    let cubes = fill_scene(&LEVELS[..5], &mut scene, &prototype);
    println!("Initialized {} cubes", cubes.len());

    let mut pass = baryon::pass::Solid::new(
        &baryon::pass::SolidConfig {
            cull_back_faces: true,
        },
        &context,
    );

    let mut moment = time::Instant::now();

    window.run(move |event| match event {
        Event::Resize { width, height } => {
            context.resize(width, height);
        }
        Event::Draw => {
            let delta = moment.elapsed().as_secs_f32();
            moment = time::Instant::now();
            for cube in cubes.iter() {
                let level = &LEVELS[cube.level as usize];
                scene[cube.node].pre_rotate(
                    mint::Vector3 {
                        x: 0.0,
                        y: 0.0,
                        z: 1.0,
                    },
                    delta * level.speed,
                );
            }

            context.present(&mut pass, &scene, &camera);
        }
        _ => {}
    })
}
