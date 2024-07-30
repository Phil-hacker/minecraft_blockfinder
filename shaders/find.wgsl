@group(0) @binding(0)
var<uniform> chunk_size: vec3<u32>;

@group(0) @binding(1)
var<storage, read_write> chunk: array<u32>;

@group(0) @binding(2)
var<storage, read> grid: array<u32>;

@group(0) @binding(3)
var<storage, read_write> position: array<u32,3>;

const FIND_SIZE: u32 = 32;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) workgroups: vec3<u32>) {
    var result = true;
    for(var x: u32 = 0; x < FIND_SIZE; x+=1u) {
        for(var y: u32 = 0; y < FIND_SIZE; y+=1u) {
            for(var z: u32 = 0; z < FIND_SIZE; z+=1u) {
                let grid_data = get_grid(to_index(vec3<u32>(FIND_SIZE), vec3<u32>(x,y,z)));
                let chunk_data = get_chunk(to_index(chunk_size, invocation_id+vec3<u32>(x,y,z)));
                if !check(grid_data, chunk_data) {
                    result = false;
                    return;
                }
            }
        }
    }
    if(result) {
        position[0] = invocation_id.x;
        position[1] = invocation_id.y;
        position[2] = invocation_id.z;
    }
    
}

fn to_index(workgroups: vec3<u32>, position: vec3<u32>) -> u32 {
    return position.y * workgroups.x * workgroups.z + position.z * workgroups.x + position.x;
}

fn check(grid_rotation: u32, chunk_rotation: u32) -> bool {
    let max_rotation = get_max_rotation(grid_rotation);
    let rotation = get_rotation(grid_rotation);
    return (max_rotation <= 1
         || (chunk_rotation % max_rotation) == rotation);
}

fn get_grid(index: u32) -> u32 {
    let real_index = index / 4;
    return (grid[real_index] >> ((index % 4) * 8)) & 255;
}

fn get_chunk(index: u32) -> u32 {
    let real_index = index / 16;
    return (chunk[real_index] >> ((index % 16) * 2)) & 3;
    //return chunk[index];
}

fn get_rotation(data: u32) -> u32 {
    return data & 15;
}

fn get_max_rotation(data: u32) -> u32 {
    return (data >> 4) & 15;
}

fn unpack4xU8(data: u32) -> vec4<u32> {
    return vec4(
        data & 255,
        (data >> 8) & 255,
        (data >> 16) & 255,
        (data >> 24) & 255
    );
}