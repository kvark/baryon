struct Globals {
    view_proj: mat4x4<f32>;
};
[[group(0), binding(0)]]
var<uniform> globals: Globals;

[[group(0), binding(1)]]
var sam: sampler;

struct Locals {
    pos_scale: vec4<f32>;
    rot: vec4<f32>;
    bounds: vec4<f32>;
    tex_coords: vec4<f32>;
};
[[group(1), binding(0)]]
var<uniform> locals: Locals;

[[group(1), binding(1)]]
var image: texture_2d<f32>;

fn qrot(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    return v + 2.0*cross(q.xyz, cross(q.xyz,v) + q.w*v);
}

struct Varyings {
    [[builtin(position)]] clip_pos: vec4<f32>;
    [[location(0)]] tc: vec2<f32>;
};

[[stage(vertex)]]
fn main_vs([[builtin(vertex_index)]] index: u32) -> Varyings {
    let tc = vec2<f32>(
        f32(i32(index) / 2),
        f32(i32(index) & 1),
    );
    let pos = vec3<f32>(
        mix(locals.bounds.xw, locals.bounds.zy, tc),
        0.0
    );
    let world = locals.pos_scale.w * qrot(locals.rot, pos) + locals.pos_scale.xyz;
    let clip_pos = globals.view_proj * vec4<f32>(world, 1.0);

    let tc_sub = mix(locals.tex_coords.xy, locals.tex_coords.zw, tc);
    return Varyings( clip_pos, tc_sub );
}

[[stage(fragment)]]
fn main_fs([[location(0)]] tc: vec2<f32>) -> [[location(0)]] vec4<f32> {
    let sample = textureSample(image, sam, tc);
    // pre-multiply the alpha
    return sample * vec4<f32>(sample.aaa, 1.0);
}
