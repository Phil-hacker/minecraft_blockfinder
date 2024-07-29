@group(0) @binding(0)
var<uniform> position: vec2<i32>;

@group(0) @binding(1)
var<storage, read_write> chunk: array<u32>;


@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) workgroups: vec3<u32>) {
    let index = to_index(workgroups, invocation_id);
    chunk[index] = get_block_rotations(vec3i64(invocation_id) * vec3(4,1,1) + vec3(i64(position.x),0,i64(position.y)));
}

fn get_block_rotations(pos: vec3<i64>) -> u32 {
    return pack4xU8(vec4(
        get_block_rotation(pos),
        get_block_rotation(pos + vec3(1,0,0)),
        get_block_rotation(pos + vec3(2,0,0)),
        get_block_rotation(pos + vec3(3,0,0)),
    ));
}


fn vec3i64(value: vec3<u32>) -> vec3<i64> {
    return vec3(i64(value.x), i64(value.y), i64(value.z));
}

fn get_block_rotation(pos: vec3<i64>) -> u32 {
    return u32(abs(get_rotation_from_seed(get_rendering_seed(pos.x, pos.y, pos.z) >> 16)) & 3);
}
fn get_rendering_seed(x: i64, y: i64, z: i64) -> i64 {
    let l = (x * 3129871) ^ z * 116129781 ^ y;
    let l2 = (l * l * 42317861) + l * 11;
    return l2;
}

fn get_rotation_from_seed(seed: i64) -> i32 {
    let seed2 = (seed ^ 0x5DEECE66D) & 0xFFFFFFFFFFFF;
    let value = i64((u64(seed2 * 0xBB20B4600A69 + 0x40942DE6BA) >> 16) );
    return i32(value);
}

// helper functions

fn to_index(workgroups: vec3<u32>, position: vec3<u32>) -> u32 {
    return position.y * workgroups.x * workgroups.z + position.z * workgroups.x + position.x;
}

fn from_index(workgroups: vec3<u32>, index: u32) -> vec3<u32> {
    let h = index / (workgroups.x * workgroups.z);
    let l = (index - h * workgroups.x * workgroups.z) / workgroups.x;
    let w = (index - h * workgroups.x * workgroups.z - l * workgroups.x);
    return vec3(w, h, l);
}

fn pack4xU8(data: vec4<u32>) -> u32 {
    return data.r | (data.g << 8) | (data.b << 16) | (data.a << 24);
}