struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) local_position: vec3<f32>,
    @location(2) world_normal: vec3<f32>,
    @location(3) world_tangent: vec4<f32>,
    @location(4) uv: vec2<f32>,
    @location(5) @interpolate(flat) texture: u32,
    @location(6) @interpolate(flat) bitfields: u32,
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(7) @interpolate(flat) instance_index: u32,
#endif
    @location(8) color: vec3<f32>,
}

struct PrepassOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) local_position: vec3<f32>,
#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
    @location(2) world_normal: vec3<f32>,
    @location(3) world_tangent: vec4<f32>,
#endif
    @location(4) uv: vec2<f32>,
    @location(5) @interpolate(flat) texture: u32,
    @location(6) @interpolate(flat) bitfields: u32,
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(7) @interpolate(flat) instance_index: u32,
#endif
#ifdef MOTION_VECTOR_PREPASS
    @location(7) previous_world_position: vec4<f32>,
#endif
#ifdef DEPTH_CLAMP_ORTHO
    @location(8) clip_position_unclamped: vec4<f32>,
#endif
}