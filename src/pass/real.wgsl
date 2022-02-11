struct Attributes {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] tex_coords: vec2<f32>;
    [[location(2)]] normal: vec3<f32>;
};

struct Varyings {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] world_pos: vec3<f32>;
    [[location(1)]] tex_coords: vec2<f32>;
    [[location(2)]] normal: vec3<f32>;
};

struct Globals {
    view_proj: mat4x4<f32>;
    camerate_pos: vec4<f32>;
};
[[group(0), binding(0)]]
var<uniform> globals: Globals;

struct Locals {
    pos_scale: vec4<f32>;
    rot: vec4<f32>;
    base_color_factor: vec4<f32>;
    emissive_factor: vec4<f32>;
    metallic_roughness_values: vec2<f32>;
    normal_scale: f32;
    occlusion_strength: f32;
};
[[group(1), binding(0)]]
var<uniform> locals: Locals;

fn qrot(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    return v + 2.0*cross(q.xyz, cross(q.xyz,v) + q.w*v);
}

[[stage(vertex)]]
fn main_vs(in: Attributes) -> Varyings {
    let world = locals.pos_scale.w * qrot(locals.rot, in.position) + locals.pos_scale.xyz;
    let normal = qrot(locals.rot, in.normal);

    return Varyings(
        globals.view_proj * vec4<f32>(world, 1.0),
        world,
        in.tex_coords,
        normal,
    );
}

let PI: f32 = 3.141592653589793;
let MIN_ROUGHNESS: f32 = 0.04;
let MAX_LIGHTS: u32 = 4u;

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

[[group(0), binding(2)]]
var sam: sampler;

[[group(1), binding(1)]]
var base_color_map: texture_2d<f32>;


struct PbrInfo {
    ndotl: f32;
    ndotv: f32;
    ndoth: f32;
    ldoth: f32;
    vdoth: f32;
    perceptual_roughness: f32;
    metalness: f32;
    base_color: vec3<f32>;
    reflectance0: vec3<f32>;
    reflectance90: vec3<f32>;
    alpha_roughness: f32;
};

fn smith(ndotv: f32, r: f32) -> f32 {
    let tan_sq = (1.0 - ndotv * ndotv) / max((ndotv * ndotv), 0.00001);
    return 2.0 / (1.0 + sqrt(1.0 + r * r * tan_sq));
}

fn geometric_occlusion_smith_ggx(pbr: PbrInfo) -> f32 {
    return smith(pbr.ndotl, pbr.alpha_roughness) * smith(pbr.ndotv, pbr.alpha_roughness);
}

// Basic Lambertian diffuse, implementation from Lambert's Photometria
// https://archive.org/details/lambertsphotome00lambgoog
fn lambertian_diffuse(pbr: PbrInfo) -> vec3<f32>{
    return pbr.base_color / PI;
}

// The following equations model the Fresnel reflectance term of the spec equation
// (aka F()) implementation of fresnel from “An Inexpensive BRDF Model for Physically
// based Rendering” by Christophe Schlick
fn fresnel_schlick(pbr: PbrInfo) -> vec3<f32> {
    return pbr.reflectance0 + (pbr.reflectance90 - pbr.reflectance0) * pow(clamp(1.0 - pbr.vdoth, 0.0, 1.0), 5.0);
}

// The following equation(s) model the distribution of microfacet normals across
// the area being drawn (aka D())
// Implementation from “Average Irregularity Representation of a Roughened Surface
// for Ray Reflection” by T. S. Trowbridge, and K. P. Reitz
fn ggx(pbr: PbrInfo) -> f32 {
    let roughness_sq = pbr.alpha_roughness * pbr.alpha_roughness;
    let f = (pbr.ndoth * roughness_sq - pbr.ndoth) * pbr.ndoth + 1.0;
    return roughness_sq / (PI * f * f);
}

[[stage(fragment)]]
fn main_fs(in: Varyings) -> [[location(0)]] vec4<f32> {
    let v = normalize(globals.camerate_pos.xyz - in.world_pos);
    let n = normalize(in.normal);

    let perceptual_roughness = clamp(locals.metallic_roughness_values.y, MIN_ROUGHNESS, 1.0);
    let metallic = clamp(locals.metallic_roughness_values.x, 0.0, 1.0);

    let base_color = locals.base_color_factor * textureSample(base_color_map, sam, in.tex_coords);

    let f0 = 0.04;
    let diffuse_color = mix(base_color.xyz * (1.0 - f0), vec3<f32>(0.0), metallic);
    let specular_color = mix(vec3<f32>(f0), base_color.xyz, metallic);
    let reflectance = max(max(specular_color.x, specular_color.y), specular_color.z);

    // For typical incident reflectance range (between 4% to 100%) set the grazing
    // reflectance to 100% for typical fresnel effect.
    // For very low reflectance range on highly diffuse objects (below 4%),
    // incrementally reduce grazing reflecance to 0%.
    let reflectance90 = clamp(reflectance * 25.0, 0.0, 1.0);
    let specular_environment_r0 = specular_color;
    let specular_environment_r90 = vec3<f32>(1.0) * reflectance90;

    // Roughness is authored as perceptual roughness; as is convention, convert to
    // material roughness by squaring the perceptual roughness
    let alpha_roughness = perceptual_roughness * perceptual_roughness;

    var color = vec3<f32>(0.0);
    let num_lights = min(MAX_LIGHTS, arrayLength(&lights.data));
    for (var i = 0u; i<num_lights; i = i + 1u) {
        let light = lights.data[i];
        let l = normalize(light.pos.xyz - light.pos.w * in.world_pos);
        let h = normalize(l + v);
        let reflection = -normalize(reflect(v, n));

        let ndotl = clamp(dot(n, l), 0.001, 1.0);
        let ndotv = abs(dot(n, v)) + 0.001;
        let ndoth = clamp(dot(n, h), 0.0, 1.0);
        let ldoth = clamp(dot(l, h), 0.0, 1.0);
        let vdoth = clamp(dot(v, h), 0.0, 1.0);
        let pbr_inputs = PbrInfo(
            ndotl,
            ndotv,
            ndoth,
            ldoth,
            vdoth,
            perceptual_roughness,
            metallic,
            diffuse_color,
            specular_environment_r0,
            specular_environment_r90,
            alpha_roughness,
        );

        let f = fresnel_schlick(pbr_inputs);
        let g = geometric_occlusion_smith_ggx(pbr_inputs);
        let d = ggx(pbr_inputs);
        let diffuse_contrib = (1.0 - f) * lambertian_diffuse(pbr_inputs);
        let spec_contrib = f * g * d / (4.0 * ndotl * ndotv);
        color = color + ndotl * light.color_intensity.w * light.color_intensity.xyz * (diffuse_contrib + spec_contrib);
    }

    return vec4<f32>(color, base_color.a);
}
