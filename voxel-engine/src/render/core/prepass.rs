use bevy::{
    asset::{AssetServer, Handle},
    core_pipeline::{
        core_3d::CORE_3D_DEPTH_FORMAT,
        prepass::{
            DepthPrepass, MotionVectorPrepass, NormalPrepass, Opaque3dPrepass,
            MOTION_VECTOR_PREPASS_FORMAT, NORMAL_PREPASS_FORMAT,
        },
    },
    ecs::{
        query::Has,
        system::{Query, Res, ResMut, Resource},
        world::{FromWorld, World},
    },
    log::error,
    pbr::{
        DrawMesh, MeshLayouts, MeshPipelineKey, PreviousViewProjection, RenderMeshInstances,
        SetMeshBindGroup, SetPrepassViewBindGroup,
    },
    render::{
        globals::GlobalsUniform,
        mesh::{Mesh, MeshVertexBufferLayout},
        render_asset::RenderAssets,
        render_phase::{DrawFunctions, RenderPhase, SetItemPipeline},
        render_resource::{
            binding_types::uniform_buffer, BindGroupLayout, BindGroupLayoutEntries,
            ColorTargetState, ColorWrites, CompareFunction, DepthBiasState, DepthStencilState,
            Face, FragmentState, FrontFace, MultisampleState, PipelineCache, PolygonMode,
            PrimitiveState, RenderPipelineDescriptor, Shader, ShaderDefVal, ShaderStages,
            SpecializedMeshPipeline, SpecializedMeshPipelineError, SpecializedMeshPipelines,
            StencilFaceState, StencilState, VertexState,
        },
        renderer::RenderDevice,
        view::{ExtractedView, ViewUniform, VisibleEntities},
    },
};

use crate::{
    data::texture::GpuFaceTexture,
    render::{
        core::{gpu_chunk::ChunkRenderData, render::ChunkPipeline},
        occlusion::ChunkOcclusionMap,
        quad::GpuQuadBitfields,
    },
};

use super::{
    gpu_chunk::{ChunkRenderDataStore, SetChunkBindGroup},
    gpu_registries::SetRegistryBindGroup,
    render::ChunkPipelineKey,
    u32_shader_def, DefaultBindGroupLayouts, RenderCore,
};

#[derive(Clone, Resource)]
pub struct ChunkPrepassPipeline {
    pub view_layout_motion_vectors: BindGroupLayout,
    pub view_layout_no_motion_vectors: BindGroupLayout,
    pub mesh_layouts: MeshLayouts,
    pub layouts: DefaultBindGroupLayouts,
    pub vert: Handle<Shader>,
    pub frag: Handle<Shader>,
}

impl FromWorld for ChunkPrepassPipeline {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();
        let gpu = world.resource::<RenderDevice>();

        let mesh_pipeline = world.resource::<ChunkPipeline>();

        let view_layout_motion_vectors = gpu.create_bind_group_layout(
            "chunk_prepass_view_layout_motion_vectors",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX_FRAGMENT,
                (
                    // View
                    uniform_buffer::<ViewUniform>(true),
                    // Globals
                    uniform_buffer::<GlobalsUniform>(false),
                    // PreviousViewProjection
                    uniform_buffer::<PreviousViewProjection>(true),
                ),
            ),
        );

        let view_layout_no_motion_vectors = gpu.create_bind_group_layout(
            "chunk_prepass_view_layout_no_motion_vectors",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX_FRAGMENT,
                (
                    // View
                    uniform_buffer::<ViewUniform>(true),
                    // Globals
                    uniform_buffer::<GlobalsUniform>(false),
                ),
            ),
        );

        Self {
            view_layout_motion_vectors,
            view_layout_no_motion_vectors,
            mesh_layouts: mesh_pipeline.mesh_pipeline.mesh_layouts.clone(),
            layouts: world.resource::<DefaultBindGroupLayouts>().clone(),
            vert: server.load("shaders/vxl_chunk_vert_prepass.wgsl"),
            frag: server.load("shaders/vxl_chunk_frag_prepass.wgsl"),
        }
    }
}

// most of this code is taken verbatim from
// https://github.com/bevyengine/bevy/blob/d4132f661a8a567fd3f9c3b329c2b4032bb1e05e/crates/bevy_pbr/src/prepass/mod.rs#L297C1-L582C2
impl SpecializedMeshPipeline for ChunkPrepassPipeline {
    type Key = ChunkPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut bind_group_layouts = vec![if key
            .mesh_key
            .contains(MeshPipelineKey::MOTION_VECTOR_PREPASS)
        {
            self.view_layout_motion_vectors.clone()
        } else {
            self.view_layout_no_motion_vectors.clone()
        }];

        bind_group_layouts.extend_from_slice(&[
            self.mesh_layouts.model_only.clone(),
            self.layouts.registry_bg_layout.clone(),
            self.layouts.chunk_bg_layout.clone(),
        ]);

        let mut shader_defs: Vec<ShaderDefVal> = vec![
            "PREPASS_PIPELINE".into(),
            "VERTEX_UVS".into(),
            "VERTEX_NORMALS".into(),
            "VERTEX_TANGENTS".into(),
            u32_shader_def("ROTATION_MASK", GpuQuadBitfields::ROTATION_MASK),
            u32_shader_def("ROTATION_SHIFT", GpuQuadBitfields::ROTATION_SHIFT),
            u32_shader_def("FACE_MASK", GpuQuadBitfields::FACE_MASK),
            u32_shader_def("FACE_SHIFT", GpuQuadBitfields::FACE_SHIFT),
            u32_shader_def("FLIP_UV_X_BIT", GpuQuadBitfields::FLIP_UV_X_BIT),
            u32_shader_def("FLIP_UV_Y_BIT", GpuQuadBitfields::FLIP_UV_Y_BIT),
            u32_shader_def(
                "CHUNK_OCCLUSION_BUFFER_SIZE",
                ChunkOcclusionMap::GPU_BUFFER_SIZE,
            ),
            u32_shader_def(
                "CHUNK_OCCLUSION_BUFFER_DIMENSIONS",
                ChunkOcclusionMap::GPU_BUFFER_DIMENSIONS,
            ),
            u32_shader_def("HAS_NORMAL_MAP_BIT", GpuFaceTexture::HAS_NORMAL_MAP_BIT),
        ];

        if key.mesh_key.contains(MeshPipelineKey::DEPTH_PREPASS) {
            shader_defs.push("DEPTH_PREPASS".into());
        }

        if key.mesh_key.contains(MeshPipelineKey::NORMAL_PREPASS) {
            shader_defs.push("NORMAL_PREPASS".into());
        }

        if key
            .mesh_key
            .intersects(MeshPipelineKey::NORMAL_PREPASS | MeshPipelineKey::DEFERRED_PREPASS)
        {
            shader_defs.push("NORMAL_PREPASS_OR_DEFERRED_PREPASS".into());
        }

        if key
            .mesh_key
            .intersects(MeshPipelineKey::MOTION_VECTOR_PREPASS | MeshPipelineKey::DEFERRED_PREPASS)
        {
            shader_defs.push("MOTION_VECTOR_PREPASS_OR_DEFERRED_PREPASS".into());
        }

        if key
            .mesh_key
            .contains(MeshPipelineKey::MOTION_VECTOR_PREPASS)
        {
            shader_defs.push("MOTION_VECTOR_PREPASS".into());
        }

        if key.mesh_key.intersects(
            MeshPipelineKey::NORMAL_PREPASS
                | MeshPipelineKey::MOTION_VECTOR_PREPASS
                | MeshPipelineKey::DEFERRED_PREPASS,
        ) {
            shader_defs.push("PREPASS_FRAGMENT".into());
        }

        if key.mesh_key.contains(MeshPipelineKey::DEPTH_CLAMP_ORTHO) {
            shader_defs.push("DEPTH_CLAMP_ORTHO".into());
            // PERF: This line forces the "prepass fragment shader" to always run in
            // common scenarios like "directional light calculation". Doing so resolves
            // a pretty nasty depth clamping bug, but it also feels a bit excessive.
            // We should try to find a way to resolve this without forcing the fragment
            // shader to run.
            // https://github.com/bevyengine/bevy/pull/8877
            shader_defs.push("PREPASS_FRAGMENT".into());
        }

        let mut targets = vec![
            key.mesh_key
                .contains(MeshPipelineKey::NORMAL_PREPASS)
                .then_some(ColorTargetState {
                    format: NORMAL_PREPASS_FORMAT,
                    // BlendState::REPLACE is not needed here, and None will be potentially much faster in some cases.
                    blend: None,
                    write_mask: ColorWrites::ALL,
                }),
            key.mesh_key
                .contains(MeshPipelineKey::MOTION_VECTOR_PREPASS)
                .then_some(ColorTargetState {
                    format: MOTION_VECTOR_PREPASS_FORMAT,
                    // BlendState::REPLACE is not needed here, and None will be potentially much faster in some cases.
                    blend: None,
                    write_mask: ColorWrites::ALL,
                }),
            // these 2 render targets are normally for the deferred prepass, but we dont support
            // deferred rendering for chunks yet so we just leave these as None for now
            None,
            None,
        ];

        if targets.iter().all(Option::is_none) {
            // if no targets are required then clear the list, so that no fragment shader is required
            // (though one may still be used for discarding depth buffer writes)
            targets.clear();
        }

        let fragment_required = !targets.is_empty()
            || key.mesh_key.contains(MeshPipelineKey::DEPTH_CLAMP_ORTHO)
            || key.mesh_key.contains(MeshPipelineKey::MAY_DISCARD);

        let fragment = fragment_required.then(|| {
            // Use the fragment shader from the material

            FragmentState {
                shader: self.frag.clone(),
                entry_point: "fragment".into(),
                shader_defs: shader_defs.clone(),
                targets,
            }
        });

        Ok(RenderPipelineDescriptor {
            label: Some("chunk_prepass_pipeline".into()),
            layout: bind_group_layouts,
            push_constant_ranges: Vec::new(),
            vertex: VertexState {
                shader: self.vert.clone(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![layout.get_layout(&[
                    RenderCore::QUAD_INDEX_ATTR.at_shader_location(0),
                    // Mesh::ATTRIBUTE_POSITION.at_shader_location(1),
                ])?],
            },
            primitive: PrimitiveState {
                topology: key.mesh_key.primitive_topology(),
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment,
        })
    }
}

pub fn queue_prepass_chunks(
    functions: Res<DrawFunctions<Opaque3dPrepass>>,
    chunk_data_store: Res<ChunkRenderDataStore>,
    mut pipelines: ResMut<SpecializedMeshPipelines<ChunkPrepassPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    prepass_pipeline: Res<ChunkPrepassPipeline>,
    render_mesh_instances: ResMut<RenderMeshInstances>,
    render_meshes: Res<RenderAssets<Mesh>>,
    mut views: Query<(
        &ExtractedView,
        &VisibleEntities,
        &mut RenderPhase<Opaque3dPrepass>,
        Has<DepthPrepass>,
        Has<NormalPrepass>,
        Has<MotionVectorPrepass>,
    )>,
) {
    let draw_function = functions.read().get_id::<DrawVoxelChunkPrepass>().unwrap();

    for (view, visible_entities, mut phase, depth_prepass, normal_prepass, motion_vector_prepass) in
        &mut views
    {
        let mut view_key = MeshPipelineKey::empty();

        if depth_prepass {
            view_key |= MeshPipelineKey::DEPTH_PREPASS;
        }
        if normal_prepass {
            view_key |= MeshPipelineKey::NORMAL_PREPASS;
        }
        if motion_vector_prepass {
            view_key |= MeshPipelineKey::MOTION_VECTOR_PREPASS;
        }

        let rangefinder = view.rangefinder3d();

        for entity in &visible_entities.entities {
            // skip all entities that dont have chunk render data
            if !chunk_data_store
                .map
                .get(entity)
                .is_some_and(|data| matches!(data, ChunkRenderData::BindGroup(_)))
            {
                continue;
            }

            let Some(mesh_instance) = render_mesh_instances.get(entity) else {
                continue;
            };

            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };

            let mesh_key =
                MeshPipelineKey::from_primitive_topology(mesh.primitive_topology) | view_key;

            let pipeline_id = match pipelines.specialize(
                &pipeline_cache,
                &prepass_pipeline,
                ChunkPipelineKey { mesh_key },
                &mesh.layout,
            ) {
                Ok(id) => id,
                Err(err) => {
                    error!("{}", err);
                    continue;
                }
            };

            let distance =
                rangefinder.distance_translation(&mesh_instance.transforms.transform.translation);

            phase.add(Opaque3dPrepass {
                entity: *entity,
                draw_function: draw_function,
                pipeline_id,
                distance,
                batch_range: 0..1,
                dynamic_offset: None,
            });
        }
    }
}

pub type DrawVoxelChunkPrepass = (
    SetItemPipeline,
    SetPrepassViewBindGroup<0>,
    SetMeshBindGroup<1>,
    SetRegistryBindGroup<2>,
    SetChunkBindGroup<3>,
    DrawMesh,
);
