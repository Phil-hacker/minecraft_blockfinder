mod gpu;

use std::time::{Duration, Instant};

use crate::constants::GRID_SIZE;

use bevy::{math::IVec3, prelude::Resource, render::extract_resource::ExtractResource};
pub use gpu::GPUFinderPlugin;

#[derive(Resource, Clone, ExtractResource)]
pub struct FinderJob(pub Box<[u8; GRID_SIZE.0 * GRID_SIZE.1 * GRID_SIZE.2]>);

#[derive(Resource, Clone, Copy, Debug)]
pub enum FinderStatus {
    WaitingForJob,
    Running { blocks: u64, start_time: Instant },
    Finished { searched_blocks: u64, pos: IVec3, time: Duration },
}
