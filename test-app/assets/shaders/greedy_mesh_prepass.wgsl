#import bevy_pbr::{
    prepass_bindings,
    mesh_functions,
    prepass_io::{VertexOutput, FragmentOutput},
    skinning,
    morph,
    mesh_view_bindings::{view, previous_view_proj},
}

#import bevy_render::instance_index::get_instance_index

#ifdef DEFERRED_PREPASS
#import bevy_pbr::rgb9e5
#endif

struct GreedyVertexOutput {
    // This is `clip position` when the struct is used as a vertex stage output
    // and `frag coord` when used as a fragment stage input
    @builtin(position) position: vec4<f32>,

#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
    @location(1) world_normal: vec3<f32>,
    @location(2) world_tangent: vec4<f32>,
#endif // NORMAL_PREPASS_OR_DEFERRED_PREPASS

    @location(3) world_position: vec4<f32>,
#ifdef MOTION_VECTOR_PREPASS
    @location(4) previous_world_position: vec4<f32>,
#endif

#ifdef DEPTH_CLAMP_ORTHO
    @location(5) clip_position_unclamped: vec4<f32>,
#endif // DEPTH_CLAMP_ORTHO
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) instance_index: u32,
#endif

#ifdef VERTEX_COLORS
    @location(7) color: vec4<f32>,
#endif
    @location(10) @interpolate(flat) texture_id: u32,
    @location(11) @interpolate(flat) texture_rot: f32,
}

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,

#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
    @location(1) normal: vec3<f32>,
#endif // NORMAL_PREPASS_OR_DEFERRED_PREPASS
}

const ROTATION_MASK: u32 = #{ROTATION_MASK}u;
const FLIP_UV_X: u32 = #{FLIP_UV_X}u;
const FLIP_UV_Y: u32 = #{FLIP_UV_Y}u;

@vertex
fn vertex(
    vertex_no_morph: Vertex, 
    @location(10) texture_id: u32,
    @location(11) misc: u32
) -> GreedyVertexOutput {
    var out: GreedyVertexOutput;
    
    var vertex = vertex_no_morph;
    out.texture_id = texture_id;

    let rotation = misc ^ ROTATION_MASK;

    out.texture_rot = radians(90.0 * f32(rotation));

    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416
    var model = mesh_functions::get_model_matrix(vertex_no_morph.instance_index);

    out.position = mesh_functions::mesh_position_local_to_clip(model, vec4(vertex.position, 1.0));

#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
    var tangent: vec3<f32>;
    if vertex.normal.y != 0.0 {
        tangent = vec3(1.0, 0.0, 0.0);
    }
    if vertex.normal.x != 0.0 {
        tangent = vec3(0.0, 0.0, 1.0);
    }
    if vertex.normal.z != 0.0 {
        tangent = vec3(1.0, 0.0 ,0.0);
    }

    if  (misc & FLIP_UV_X) != 0u {
        tangent = -tangent;
    }

    let a = out.texture_rot;
    var M: mat3x3<f32>;
    if vertex.normal.y != 0.0 {
        M = mat3x3(
            cos(a), 0.0, -sin(a),
            0.0,    1.0,     0.0,
            sin(a), 0.0,  cos(a),
        );
    }
    if vertex.normal.x != 0.0 {
        M = mat3x3(
            1.0,    0.0,     0.0,
            0.0, cos(a), -sin(a),
            0.0, sin(a),  cos(a),
        );
    }
    if vertex.normal.z != 0.0 {
        M = mat3x3(
            cos(a), -sin(a), 0.0,
            sin(a),  cos(a), 0.0,
            0.0,        0.0, 1.0,
        );
    }

    tangent = M * tangent;

    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        model,
        vec4(tangent.xyz, 0.0),
        // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
        // See https://github.com/gfx-rs/naga/issues/2416
        get_instance_index(vertex_no_morph.instance_index)
    );
#endif

#ifdef DEPTH_CLAMP_ORTHO
    out.clip_position_unclamped = out.position;
    out.position.z = min(out.position.z, 1.0);
#endif // DEPTH_CLAMP_ORTHO


#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
        // See https://github.com/gfx-rs/naga/issues/2416
        get_instance_index(vertex_no_morph.instance_index)
    );
#endif // NORMAL_PREPASS_OR_DEFERRED_PREPASS

#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif

#ifdef MOTION_VECTOR_PREPASS_OR_DEFERRED_PREPASS
    out.world_position = mesh_functions::mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
#endif // MOTION_VECTOR_PREPASS_OR_DEFERRED_PREPASS

#ifdef MOTION_VECTOR_PREPASS
    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416
    out.previous_world_position = mesh_functions::mesh_position_local_to_world(
        mesh_functions::get_previous_model_matrix(vertex_no_morph.instance_index),
        vec4<f32>(vertex.position, 1.0)
    );
#endif // MOTION_VECTOR_PREPASS

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416
    out.instance_index = get_instance_index(vertex_no_morph.instance_index);
#endif

    return out;
}

#ifdef PREPASS_FRAGMENT
@fragment
fn fragment(
    in: GreedyVertexOutput
) -> FragmentOutput {
    var out: FragmentOutput;

#ifdef NORMAL_PREPASS
    out.normal = vec4(in.world_normal * 0.5 + vec3(0.5), 1.0);
#endif

#ifdef DEPTH_CLAMP_ORTHO
    out.frag_depth = in.clip_position_unclamped.z;
#endif // DEPTH_CLAMP_ORTHO

#ifdef MOTION_VECTOR_PREPASS
    let clip_position_t = view.unjittered_view_proj * in.world_position;
    let clip_position = clip_position_t.xy / clip_position_t.w;
    let previous_clip_position_t = prepass_bindings::previous_view_proj * in.previous_world_position;
    let previous_clip_position = previous_clip_position_t.xy / previous_clip_position_t.w;
    // These motion vectors are used as offsets to UV positions and are stored
    // in the range -1,1 to allow offsetting from the one corner to the
    // diagonally-opposite corner in UV coordinates, in either direction.
    // A difference between diagonally-opposite corners of clip space is in the
    // range -2,2, so this needs to be scaled by 0.5. And the V direction goes
    // down where clip space y goes up, so y needs to be flipped.
    out.motion_vector = (clip_position - previous_clip_position) * vec2(0.5, -0.5);
#endif // MOTION_VECTOR_PREPASS

#ifdef DEFERRED_PREPASS
    // There isn't any material info available for this default prepass shader so we are just writing 
    // emissive magenta out to the deferred gbuffer to be rendered by the first deferred lighting pass layer.
    // The is here so if the default prepass fragment is used for deferred magenta will be rendered, and also
    // as an example to show that a user could write to the deferred gbuffer if they were to start from this shader.
    out.deferred = vec4(0u, bevy_pbr::rgb9e5::vec3_to_rgb9e5_(vec3(1.0, 0.0, 1.0)), 0u, 0u);
    out.deferred_lighting_pass_id = 1u;
#endif

    return out;
}
#endif // PREPASS_FRAGMENT