struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex fn vs(@builtin(vertex_index) i: u32) -> Vertex {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return Vertex(vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0), uv);
}

@group(0) @binding(0) var depth: texture_depth_2d;
@group(0) @binding(1) var depth_msaa: texture_depth_multisampled_2d;
// Inverse near, inverse far, inverse frustum width at unit distance.
@group(0) @binding(2) var<uniform> camera: vec4<f32>;

fn disparity(d: f32) -> f32 {
    // Parallel eyes 4.5 game units apart, shifted to converge at 220 units. Positive
    // disparity moves the left eye's image to the right. Limit close-up separation.
    let inverse_z = mix(camera.x, camera.y, d);
    return clamp(2.25 * camera.z * (inverse_z - 1.0 / 220.0), -0.0125, 0.0125);
}

@fragment fn save(v: Vertex) -> @location(0) f32 {
    return disparity(textureLoad(depth, vec2<i32>(v.position.xy), 0));
}

@fragment fn save_msaa(v: Vertex) -> @location(0) f32 {
    // Sample zero, like the ambient occlusion pass. Never average depths at edges.
    return disparity(textureLoad(depth_msaa, vec2<i32>(v.position.xy), 0));
}

@group(0) @binding(3) var shifts: texture_2d<f32>;
@group(0) @binding(4) var colour: texture_2d<f32>;
@group(0) @binding(5) var linear: sampler;
@group(0) @binding(6) var<uniform> ramp: array<vec4<u32>, 256>;

fn shift_at(uv: vec2<f32>) -> f32 {
    let dims = vec2<i32>(textureDimensions(shifts));
    return textureLoad(shifts, clamp(vec2<i32>(uv * vec2<f32>(dims)), vec2<i32>(0), dims - 1), 0).r;
}

fn eye(uv: vec2<f32>, direction: f32) -> vec3<f32> {
    // Invert the horizontal projection. At a disocclusion there is no matching source
    // sample; prefer the farther of the two estimates instead of stretching the occluder.
    let a = uv - vec2<f32>(direction * shift_at(uv), 0.0);
    let da = shift_at(a);
    let b = uv - vec2<f32>(direction * da, 0.0);
    let db = shift_at(b);
    let stable = abs(db - da) * f32(textureDimensions(shifts).x) <= 0.5;
    let source = select(a, b, stable || db < da);
    return textureSampleLevel(colour, linear, source, 0.0).rgb;
}

fn show(rgb: vec3<f32>) -> vec4<f32> {
    // Same 8-bit lookup as the regular window blit (ramp.rs).
    let k = vec3<u32>(round(saturate(rgb) * 255.0));
    let w = vec3<u32>(ramp[k.r].x, ramp[k.g].y, ramp[k.b].z);
    return vec4<f32>(vec3<f32>((w * 255u + 32767u) / 65535u) / 255.0, 1.0);
}

@fragment fn anaglyph(v: Vertex) -> @location(0) vec4<f32> {
    // Luma in both eyes keeps a brightly saturated object visible through either lens.
    let weights = vec3<f32>(0.2126, 0.7152, 0.0722);
    let red = dot(eye(v.uv, 1.0), weights);
    let cyan = dot(eye(v.uv, -1.0), weights);
    return show(vec3<f32>(red, cyan, cyan));
}

fn paired_uv(uv: vec2<f32>) -> vec2<f32> {
    // Each view is half the window width AND height, vertically centred: no squeeze.
    return vec2<f32>(fract(uv.x * 2.0), uv.y * 2.0 - 0.5);
}

@fragment fn crossed(v: Vertex) -> @location(0) vec4<f32> {
    let uv = paired_uv(v.uv);
    if uv.y < 0.0 || uv.y >= 1.0 { return vec4<f32>(0.0, 0.0, 0.0, 1.0); }
    return show(eye(uv, select(-1.0, 1.0, v.uv.x >= 0.5)));
}

@fragment fn crossed_flat(v: Vertex) -> @location(0) vec4<f32> {
    let uv = paired_uv(v.uv);
    if uv.y < 0.0 || uv.y >= 1.0 { return vec4<f32>(0.0, 0.0, 0.0, 1.0); }
    return show(textureSampleLevel(colour, linear, uv, 0.0).rgb);
}
