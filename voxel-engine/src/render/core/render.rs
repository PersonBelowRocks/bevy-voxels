use bevy::{
    asset::{AssetServer, Handle},
    core_pipeline::{
        core_3d::Opaque3d,
        prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
        tonemapping::{DebandDither, Tonemapping},
    },
    ecs::{
        query::Has,
        system::{Query, Res, ResMut, Resource},
        world::{FromWorld, World},
    },
    log::error,
    pbr::{
        DrawMesh, MeshPipeline, MeshPipelineKey, RenderMeshInstances,
        ScreenSpaceAmbientOcclusionSettings, SetMeshBindGroup, SetMeshViewBindGroup,
        ShadowFilteringMethod,
    },
    render::{
        camera::{Projection, TemporalJitter},
        mesh::{Mesh, MeshVertexBufferLayout},
        render_asset::RenderAssets,
        render_phase::{DrawFunctions, RenderPhase, SetItemPipeline},
        render_resource::{
            BindGroupLayout, Face, FrontFace, PipelineCache, RenderPipelineDescriptor, Shader,
            SpecializedMeshPipeline, SpecializedMeshPipelineError, SpecializedMeshPipelines,
        },
        view::{ExtractedView, VisibleEntities},
    },
};

use crate::{
    data::texture::GpuFaceTexture,
    render::{occlusion::ChunkOcclusionMap, quad::GpuQuadBitfields},
};

use super::{
    gpu_chunk::{ChunkRenderData, ChunkRenderDataStore, SetChunkBindGroup},
    gpu_registries::SetRegistryBindGroup,
    u32_shader_def, DefaultBindGroupLayouts, RenderCore,
};

#[derive(Resource, Clone)]
pub struct ChunkPipeline {
    pub mesh_pipeline: MeshPipeline,
    pub registry_layout: BindGroupLayout,
    pub chunk_layout: BindGroupLayout,
    pub vert: Handle<Shader>,
    pub frag: Handle<Shader>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChunkPipelineKey {
    pub mesh_key: MeshPipelineKey,
}

impl FromWorld for ChunkPipeline {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();

        let layouts = world.resource::<DefaultBindGroupLayouts>();

        Self {
            mesh_pipeline: world.resource::<MeshPipeline>().clone(),
            registry_layout: layouts.registry_bg_layout.clone(),
            chunk_layout: layouts.chunk_bg_layout.clone(),
            vert: server.load("shaders/vxl_chunk_vert.wgsl"),
            frag: server.load("shaders/vxl_chunk_frag.wgsl"),
        }
    }
}

impl SpecializedMeshPipeline for ChunkPipeline {
    type Key = ChunkPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key.mesh_key, layout)?;
        descriptor.label = Some("chunk_pipeline".into());

        descriptor.primitive.cull_mode = Some(Face::Back);
        descriptor.primitive.front_face = FrontFace::Ccw;

        descriptor.vertex.shader = self.vert.clone();
        descriptor.vertex.buffers =
            vec![layout.get_layout(&[RenderCore::QUAD_INDEX_ATTR.at_shader_location(0)])?];
        descriptor.fragment.as_mut().unwrap().shader = self.frag.clone();

        let shader_constants = [
            u32_shader_def("ROTATION_MASK", GpuQuadBitfields::ROTATION_MASK),
            u32_shader_def("ROTATION_SHIFT", GpuQuadBitfields::ROTATION_SHIFT),
            u32_shader_def("FACE_MASK", GpuQuadBitfields::FACE_MASK),
            u32_shader_def("FACE_SHIFT", GpuQuadBitfields::FACE_SHIFT),
            u32_shader_def("FLIP_UV_X_BIT", GpuQuadBitfields::FLIP_UV_X_BIT),
            u32_shader_def("FLIP_UV_Y_BIT", GpuQuadBitfields::FLIP_UV_Y_BIT),
            u32_shader_def("HAS_NORMAL_MAP_BIT", GpuFaceTexture::HAS_NORMAL_MAP_BIT),
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

        descriptor
            .vertex
            .shader_defs
            .extend_from_slice(&shader_constants);
        descriptor
            .fragment
            .as_mut()
            .unwrap()
            .shader_defs
            .extend_from_slice(&shader_constants);

        descriptor.layout = vec![
            self.mesh_pipeline
                .get_view_layout(key.mesh_key.into())
                .clone(),
            self.mesh_pipeline.mesh_layouts.model_only.clone(),
            self.registry_layout.clone(),
            self.chunk_layout.clone(),
        ];

        Ok(descriptor)
    }
}

pub const fn tonemapping_pipeline_key(tonemapping: Tonemapping) -> MeshPipelineKey {
    match tonemapping {
        Tonemapping::None => MeshPipelineKey::TONEMAP_METHOD_NONE,
        Tonemapping::Reinhard => MeshPipelineKey::TONEMAP_METHOD_REINHARD,
        Tonemapping::ReinhardLuminance => MeshPipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE,
        Tonemapping::AcesFitted => MeshPipelineKey::TONEMAP_METHOD_ACES_FITTED,
        Tonemapping::AgX => MeshPipelineKey::TONEMAP_METHOD_AGX,
        Tonemapping::SomewhatBoringDisplayTransform => {
            MeshPipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM
        }
        Tonemapping::TonyMcMapface => MeshPipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE,
        Tonemapping::BlenderFilmic => MeshPipelineKey::TONEMAP_METHOD_BLENDER_FILMIC,
    }
}

pub fn queue_chunks(
    functions: Res<DrawFunctions<Opaque3d>>,
    pipeline: Res<ChunkPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<ChunkPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    mut render_mesh_instances: ResMut<RenderMeshInstances>,
    render_meshes: Res<RenderAssets<Mesh>>,
    chunk_data_store: Res<ChunkRenderDataStore>,
    mut views: Query<(
        &ExtractedView,
        &VisibleEntities,
        &mut RenderPhase<Opaque3d>,
        Option<&Tonemapping>,
        Option<&DebandDither>,
        Option<&ShadowFilteringMethod>,
        Option<&Projection>,
        (
            Has<NormalPrepass>,
            Has<DepthPrepass>,
            Has<MotionVectorPrepass>,
            Has<DeferredPrepass>,
        ),
        Has<ScreenSpaceAmbientOcclusionSettings>,
        Has<TemporalJitter>,
    )>,
) {
    let draw_chunk = functions.read().id::<DrawVoxelChunk>();

    for (
        view,
        visible_entities,
        mut phase,
        tonemapping,
        dither,
        shadow_filter_method,
        projection,
        (normal_prepass, depth_prepass, motion_vector_prepass, deferred_prepass),
        ssao,
        temporal_jitter,
    ) in views.iter_mut()
    {
        let mut view_key = MeshPipelineKey::from_hdr(view.hdr);

        if normal_prepass {
            view_key |= MeshPipelineKey::NORMAL_PREPASS;
        }

        if depth_prepass {
            view_key |= MeshPipelineKey::DEPTH_PREPASS;
        }

        if motion_vector_prepass {
            view_key |= MeshPipelineKey::MOTION_VECTOR_PREPASS;
        }

        if deferred_prepass {
            view_key |= MeshPipelineKey::DEFERRED_PREPASS;
        }

        if temporal_jitter {
            view_key |= MeshPipelineKey::TEMPORAL_JITTER;
        }

        if ssao {
            view_key |= MeshPipelineKey::SCREEN_SPACE_AMBIENT_OCCLUSION;
        }

        if let Some(projection) = projection {
            view_key |= match projection {
                Projection::Perspective(_) => MeshPipelineKey::VIEW_PROJECTION_PERSPECTIVE,
                Projection::Orthographic(_) => MeshPipelineKey::VIEW_PROJECTION_ORTHOGRAPHIC,
            };
        }

        match shadow_filter_method.unwrap_or(&ShadowFilteringMethod::default()) {
            ShadowFilteringMethod::Hardware2x2 => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_HARDWARE_2X2;
            }
            ShadowFilteringMethod::Castano13 => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_CASTANO_13;
            }
            ShadowFilteringMethod::Jimenez14 => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_JIMENEZ_14;
            }
        }

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                view_key |= MeshPipelineKey::TONEMAP_IN_SHADER;
                view_key |= tonemapping_pipeline_key(*tonemapping);
            }
            if let Some(DebandDither::Enabled) = dither {
                view_key |= MeshPipelineKey::DEBAND_DITHER;
            }
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

            let Some(mesh_instance) = render_mesh_instances.get_mut(entity) else {
                continue;
            };
            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };

            let mut mesh_key = view_key;

            mesh_key |= MeshPipelineKey::from_primitive_topology(mesh.primitive_topology);

            let pipeline_id = match pipelines.specialize(
                pipeline_cache.as_ref(),
                pipeline.as_ref(),
                ChunkPipelineKey { mesh_key },
                &mesh.layout,
            ) {
                Ok(id) => id,
                Err(err) => {
                    error!("Error during voxel chunk pipeline specialization: {err}");
                    continue;
                }
            };

            let distance =
                rangefinder.distance_translation(&mesh_instance.transforms.transform.translation);

            // queue this entity for rendering
            phase.add(Opaque3d {
                entity: *entity,
                draw_function: draw_chunk,
                pipeline: pipeline_id,
                distance,
                batch_range: 0..1,
                dynamic_offset: None,
            });
        }
    }
}

pub type DrawVoxelChunk = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    SetRegistryBindGroup<2>,
    SetChunkBindGroup<3>,
    DrawMesh,
);
