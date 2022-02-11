struct Vertex {
    [[location(0)]] pos: vec3<f32>;
};

struct Globals {
    view_proj: mat4x4<f32>;
};
[[group(0), binding(0)]]
var<uniform> globals: Globals;

struct Locals {
    pos_scale: vec4<f32>;
    rot: vec4<f32>;
    color: vec4<f32>;
};
[[group(1), binding(0)]]
var<uniform> locals: Locals;

fn qrot(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    return v + 2.0*cross(q.xyz, cross(q.xyz,v) + q.w*v);
}

[[stage(vertex)]]
fn main_vs(in: Vertex) -> [[builtin(position)]] vec4<f32> {
    let world = locals.pos_scale.w * qrot(locals.rot, in.pos) + locals.pos_scale.xyz;
    return globals.view_proj * vec4<f32>(world, 1.0);
}

[[stage(fragment)]]
fn main_fs() -> [[location(0)]] vec4<f32> {
    return locals.color;
}
