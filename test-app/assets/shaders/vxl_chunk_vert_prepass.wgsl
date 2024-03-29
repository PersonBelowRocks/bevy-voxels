#import "shaders/vxl_chunk_io.wgsl"::PrepassOutput
#import "shaders/chunk_bindings.wgsl"::quads
#import "shaders/utils.wgsl"::extract_normal
#import "shaders/utils.wgsl"::extract_position
#import "shaders/utils.wgsl"::project_to_2d
#import "shaders/utils.wgsl"::axis_from_face
#import "shaders/utils.wgsl"::extract_face
#import bevy_pbr::{
    mesh_functions, 
    view_transformations::position_world_to_clip
}

@vertex
fn vertex(
    @builtin(vertex_index) vertex: u32,
    @builtin(instance_index) instance_index: u32,
    @location(0) chunk_quad_index: u32,
    // @location(1) vertex_position: vec3<f32>,
) -> PrepassOutput {

    let quad = quads[chunk_quad_index];
    var position = extract_position(quad, vertex % 4u);
    // var position = vertex_position;
    let face = extract_face(quad);
    let model = mesh_functions::get_model_matrix(instance_index);

    var out: PrepassOutput;
    out.quad_idx = chunk_quad_index;

    out.uv = project_to_2d(position, axis_from_face(face)) - quad.min;

    out.position = mesh_functions::mesh_position_local_to_clip(model, vec4(position, 1.0));
    out.local_position = position;
    out.world_position = mesh_functions::mesh_position_local_to_world(model, vec4<f32>(position, 1.0));
    
#ifdef DEPTH_CLAMP_ORTHO
    out.clip_position_unclamped = out.position;
    out.position.z = min(out.position.z, 1.0);
#endif // DEPTH_CLAMP_ORTHO

    out.instance_index = instance_index;
    
    return out;
}
