#import bevy_pbr::{
    pbr_functions::alpha_discard,
    pbr_fragment::pbr_input_from_standard_material,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
}
#endif

#import "shaders/greedy_mesh_utils.wgsl"::FaceTexture
#import "shaders/greedy_mesh_utils.wgsl"::GreedyVertexOutput
#import "shaders/greedy_mesh_utils.wgsl"::preprocess_greedy_vertex_output
#import "shaders/greedy_mesh_utils.wgsl"::greedy_mesh_pbr_input
#import "shaders/greedy_mesh_utils.wgsl"::occlusion_curve

@group(2) @binding(100)
var<storage, read> faces: array<FaceTexture>;
@group(2) @binding(101)
var<storage, read> occlusion: array<u32, #{OCCLUSION_BUFFER_SIZE}>;

@fragment
fn fragment(
    raw_in: GreedyVertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {

    let in: GreedyVertexOutput = preprocess_greedy_vertex_output(raw_in);

    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = greedy_mesh_pbr_input(in, is_front, faces[in.texture_id], 16.0);

    // alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

#ifdef PREPASS_PIPELINE
    // write the gbuffer, lighting pass id, and optionally normal and motion_vector textures
    let out = deferred_output(in, pbr_input);
#else
    // in forward mode, we calculate the lit color immediately, and then apply some post-lighting effects here.
    // in deferred mode the lit color and these effects will be calculated in the deferred lighting shader
    var out: FragmentOutput;
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

    let occlusion = occlusion_curve(in.occlusion);
    out.color = vec4(occlusion, 0.0, 0.0, 1.0);

    return out;
}