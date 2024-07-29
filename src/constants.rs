pub const AIR: crate::game_assets::BlockId = crate::game_assets::BlockId(usize::MAX, 0);

pub const VOXEL_DIMS: [f32; 3] = [1.0; 3];
pub const VOXEL_CENTER: [f32; 3] = [0.0; 3];
pub const GRID_SIZE: bevy_meshem::Dimensions = (32, 32, 32);
pub const CHUNK_SIZE: usize = 1240;
pub const CHUNK_MARGIN: usize = 40;
pub const WORLD_HEIGHT: usize = 320;

pub type Chunk = [u8; CHUNK_SIZE * CHUNK_SIZE * WORLD_HEIGHT];

const _: () = match CHUNK_SIZE % 4 == 0 {
    true => (),
    false => panic!("CHUNK_SIZE needs to be a multiple of 4"),
};