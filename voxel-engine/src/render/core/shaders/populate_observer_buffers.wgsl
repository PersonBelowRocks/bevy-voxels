#import vxl::types::{
    IndexedIndirectArgs,
    ChunkInstanceData,
    GpuChunkMetadata,
    empty_instance_data,
    instance_data_from_metadata,
    empty_indexed_indirect_args,
    indexed_args_from_metadata_and_instance,
}

@group(0) @location(0) var<storage, read_only> all_metadata: array<GpuChunkMetadata>;
@group(0) @location(1) var<storage, read_only> metadata_indices: array<u32>;

@group(1) @location(0) var<storage, write_only> instance_data: array<ChunkInstanceData>;
@group(1) @location(1) var<storage, write_only> indirect_args: array<IndexedIndirectArgs>;

@compute @workgroup_size(64)
fn populate_buffers(
    @builtin(global_invocation_id) id: vec3<u32>
) {
    let index = id.z;
    instance_data[index] = empty_instance_data();
    indirect_args[index] = empty_indexed_indirect_args();

    if arrayLength(metadata_indices) <= index {
        return;
    }

    let metadata_index = metadata_indices[index];
    let metadata = all_metadata[metadata_index];

    instance_data[index] = instance_data_from_metadata(metadata);
    // Metadata index is the same as the instance number
    indirect_args[index] = indexed_args_from_metadata_and_instance(metadata, metadata_index);
}