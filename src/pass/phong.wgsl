struct Vertex {
    [[location(0)]] pos: vec3<f32>;
    [[location(1)]] normal: vec3<f32>;
};

struct Globals {
    view_proj: mat4x4<f32>;
    ambient: vec4<f32>;
};
[[group(0), binding(0)]]
var<uniform> globals: Globals;

struct Light {
    pos: vec4<f32>;
    rot: vec4<f32>;
    color_intensity: vec4<f32>;
};
struct LightArray {
    data: array<Light>;
};
[[group(0), binding(1)]]
var<storage> lights: LightArray;

struct Locals {
    pos_scale: vec4<f32>;
    rot: vec4<f32>;
    color: vec4<f32>;
    lights: vec4<u32>;
    glossiness: f32;
};
[[group(1), binding(0)]]
var<uniform> locals: Locals;

fn qrot(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    return v + 2.0*cross(q.xyz, cross(q.xyz,v) + q.w*v);
}

struct PhongVaryings {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] world: vec3<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] color: vec3<f32>;
    [[location(3)]] half_vec0: vec3<f32>;
    [[location(4)]] half_vec1: vec3<f32>;
    [[location(5)]] half_vec2: vec3<f32>;
    [[location(6)]] half_vec3: vec3<f32>;
};

fn compute_half(world: vec3<f32>, normal: vec3<f32>, index: u32) -> vec3<f32> {
    let light_pos = lights.data[index].pos;
    let dir = light_pos.xyz - light_pos.w * world;
    return normalize(normal + normalize(dir));
}

[[stage(vertex)]]
fn vs_phong(in: Vertex) -> PhongVaryings {
    let world = locals.pos_scale.w * qrot(locals.rot, in.pos) + locals.pos_scale.xyz;
    let normal = qrot(locals.rot, normalize(in.normal));

    var out: PhongVaryings;
    out.position = globals.view_proj * vec4<f32>(world, 1.0);
    out.world = world;
    out.normal = normal;
    out.color = globals.ambient.xyz;
    out.half_vec0 = compute_half(world, normal, locals.lights.x);
    out.half_vec1 = compute_half(world, normal, locals.lights.y);
    out.half_vec2 = compute_half(world, normal, locals.lights.z);
    out.half_vec3 = compute_half(world, normal, locals.lights.w);
    return out;
}

struct Evaluation {
    diffuse: vec3<f32>;
    specular: vec3<f32>;
};

fn evaluate(world: vec3<f32>, normal: vec3<f32>, half_vec: vec3<f32>, index: u32) -> Evaluation {
    var ev = Evaluation(vec3<f32>(0.0), vec3<f32>(0.0));
    let light = lights.data[index];

    let dir = light.pos.xyz - light.pos.w * world;
    let dot_nl = dot(normal, normalize(dir));

    let kd = light.color_intensity.w * max(0.0, dot_nl);
    ev.diffuse = kd * light.color_intensity.xyz;

    if (light.color_intensity.w > 0.01 && dot_nl > 0.0) {
        let ks = dot(normal, normalize(half_vec));
        if (ks > 0.0) {
            ev.specular = pow(ks, locals.glossiness) * light.color_intensity.xyz;
        }
    }

    return ev;
}

[[stage(fragment)]]
fn fs_phong(in: PhongVaryings) -> [[location(0)]] vec4<f32> {
    let eval0 = evaluate(in.world, in.normal, in.half_vec0, locals.lights.x);
    let eval1 = evaluate(in.world, in.normal, in.half_vec1, locals.lights.y);
    let eval2 = evaluate(in.world, in.normal, in.half_vec2, locals.lights.z);
    let eval3 = evaluate(in.world, in.normal, in.half_vec3, locals.lights.w);
    let total = Evaluation(
        in.color + eval0.diffuse + eval1.diffuse + eval2.diffuse + eval3.diffuse,
        eval0.specular + eval1.specular + eval2.specular + eval3.specular,
    );
    return vec4<f32>(total.diffuse, 0.0) * locals.color + vec4<f32>(total.specular, 0.0);
}

fn evaluate_flat(world: vec3<f32>, normal: vec3<f32>, index: u32) -> vec3<f32> {
    let light = lights.data[index];

    let dir = light.pos.xyz - light.pos.w * world;
    let dot_nl = dot(normal, normalize(dir));

    let kd = light.color_intensity.w * max(0.0, dot_nl);
    return kd * light.color_intensity.xyz;
}

struct FlatVaryings {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0), interpolate(flat)]] flat_color: vec3<f32>;
    [[location(1)]] color: vec3<f32>;
};

[[stage(vertex)]]
fn vs_flat(in: Vertex) -> FlatVaryings {
    let world = locals.pos_scale.w * qrot(locals.rot, in.pos) + locals.pos_scale.xyz;
    let normal = qrot(locals.rot, normalize(in.normal));
    let diffuse = globals.ambient.xyz +
        evaluate_flat(world, normal, locals.lights.x) +
        evaluate_flat(world, normal, locals.lights.y) +
        evaluate_flat(world, normal, locals.lights.z) +
        evaluate_flat(world, normal, locals.lights.w);

    var out: FlatVaryings;
    out.position = globals.view_proj * vec4<f32>(world, 1.0);
    out.flat_color = diffuse * locals.color.xyz;
    out.color = diffuse * locals.color.xyz;
    return out;
}

[[stage(fragment)]]
fn fs_flat(in: FlatVaryings) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(in.flat_color, 0.0);
}

[[stage(fragment)]]
fn fs_gouraud(in: FlatVaryings) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(in.color, 0.0);
}
