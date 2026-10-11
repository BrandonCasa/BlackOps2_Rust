override supports_indirect_first_instance: bool;
override write_d3d12_special_constants: bool;

struct MetadataEntry {
    // bits 0..30 are an offset into `src`
    // bit 31 signifies that we are validating an indexed draw
    src_offset: u32,
    // bits 0..30 are an offset into `dst`
    // bit 30 is the most significant bit of `vertex_or_index_limit`
    // bit 31 is the most significant bit of `instance_limit`
    dst_offset: u32,
    vertex_or_index_limit: u32,
    instance_limit: u32,
    count: u32,
    first_draw: u32,
    count_offset: u32,
    workgroup_end: u32,
}
const OFFSET_MASK: u32 = 0x3FFFFFFF;

struct MetadataRange {
    start: u32,
    count: u32,
}
var<immediate> metadata_range: MetadataRange;

@group(0) @binding(0)
var<storage, read> metadata: array<MetadataEntry>;
@group(1) @binding(0)
var<storage, read> src: array<u32>;
@group(2) @binding(0)
var<storage, read_write> dst: array<u32>;
@group(3) @binding(0)
var<storage, read> counts: array<u32>;

fn is_bit_set(data: u32, index: u32) -> bool {
    return ((data >> index) & 1u) == 1u;
}

fn find_range(group: u32) -> u32 {
    var low = 0u;
    var high = metadata_range.count;
    while low < high {
        let middle = low + (high - low) / 2u;
        if group < metadata[metadata_range.start + middle].workgroup_end {
            high = middle;
        } else {
            low = middle + 1u;
        }
    }
    return metadata_range.start + low;
}

fn draw_index(entry: u32, group: u32, lane: u32) -> u32 {
    var first_group = 0u;
    if entry > metadata_range.start {
        first_group = metadata[entry - 1u].workgroup_end;
    }
    return (group - first_group) * 64u + lane;
}

@compute @workgroup_size(64)
fn main(@builtin(workgroup_id) group: vec3u, @builtin(local_invocation_index) lane: u32) {
    let entry = find_range(group.x);
    let index = draw_index(entry, group.x, lane);
    if index < metadata[entry].count {
        validate(metadata[entry], index);
    }
}

@compute @workgroup_size(64)
fn counted_main(@builtin(workgroup_id) group: vec3u, @builtin(local_invocation_index) lane: u32) {
    let entry = find_range(group.x);
    let index = draw_index(entry, group.x, lane);
    let range = metadata[entry];
    let original_count = counts[range.count_offset];
    let count = min(original_count - min(original_count, range.first_draw), range.count);
    if index == 0u {
        dst[(range.dst_offset & OFFSET_MASK) - 1u] = count;
    }
    if index < count {
        validate(range, index);
    }
}

fn validate(metadata: MetadataEntry, index: u32) {
    var failed = false;
    let is_indexed = is_bit_set(metadata.src_offset, 31);
    let src_stride = 4u + u32(is_indexed);
    let dst_stride = src_stride + select(0u, 3u, write_d3d12_special_constants);
    let src_base_offset = (metadata.src_offset & OFFSET_MASK) + index * src_stride;
    let dst_base_offset = (metadata.dst_offset & OFFSET_MASK) + index * dst_stride;

    let first_vertex_or_index = src[src_base_offset + 2];
    let vertex_or_index_count = src[src_base_offset + 0];

    {
        let overflow_permitted = is_bit_set(metadata.dst_offset, 30);
        let sub_overflows = metadata.vertex_or_index_limit < first_vertex_or_index;
        failed |= sub_overflows && !overflow_permitted;
        let vertex_or_index_limit = metadata.vertex_or_index_limit - first_vertex_or_index;
        failed |= vertex_or_index_limit < vertex_or_index_count;
    }

    let first_instance = src[src_base_offset + 3 + u32(is_indexed)];
    let instance_count = src[src_base_offset + 1];

    {
        let overflow_permitted = is_bit_set(metadata.dst_offset, 31);
        let sub_overflows = metadata.instance_limit < first_instance;
        failed |= sub_overflows && !overflow_permitted;
        let instance_limit = metadata.instance_limit - first_instance;
        failed |= instance_limit < instance_count;
    }

    if !supports_indirect_first_instance {
        failed |= first_instance != 0u;
    }

    let dst_offset = select(0u, 3u, write_d3d12_special_constants);
    if failed {
        if write_d3d12_special_constants {
            dst[dst_base_offset + 0] = 0u;
            dst[dst_base_offset + 1] = 0u;
            dst[dst_base_offset + 2] = 0u;
        }
        dst[dst_base_offset + dst_offset + 0] = 0u;
        dst[dst_base_offset + dst_offset + 1] = 0u;
        dst[dst_base_offset + dst_offset + 2] = 0u;
        dst[dst_base_offset + dst_offset + 3] = 0u;
        if (is_indexed) {
            dst[dst_base_offset + dst_offset + 4] = 0u;
        }
    } else {
        if write_d3d12_special_constants {
            dst[dst_base_offset + 0] = src[src_base_offset + 2 + u32(is_indexed)];
            dst[dst_base_offset + 1] = src[src_base_offset + 3 + u32(is_indexed)];
            dst[dst_base_offset + 2] = 0u;
        }
        dst[dst_base_offset + dst_offset + 0] = src[src_base_offset + 0];
        dst[dst_base_offset + dst_offset + 1] = src[src_base_offset + 1];
        dst[dst_base_offset + dst_offset + 2] = src[src_base_offset + 2];
        dst[dst_base_offset + dst_offset + 3] = src[src_base_offset + 3];
        if (is_indexed) {
            dst[dst_base_offset + dst_offset + 4] = src[src_base_offset + 4];
        }
    }
}
