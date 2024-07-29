use std::{
    fmt::Debug,
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
    time::Instant,
};

use crate::{constants::*, finder::util::spiral};

use bevy::log::info;

use bevy_meshem::prelude::three_d_cords;

use super::util::get_block_rotation;

pub fn create_box<T: Default + Debug, const N: usize>() -> Box<[T; N]> {
    let mut array = Vec::with_capacity(N);
    for _ in 0..N {
        array.push(T::default());
    }
    array.into_boxed_slice().try_into().unwrap()
}

pub fn generate_grid(start_x: i64, start_z: i64) -> Box<Chunk> {
    let start = Instant::now();
    let mut data = create_box::<u8, { CHUNK_SIZE * CHUNK_SIZE * WORLD_HEIGHT }>();
    for i in 0..(CHUNK_SIZE * CHUNK_SIZE * WORLD_HEIGHT) {
        let (x, y, z) = three_d_cords(i, (CHUNK_SIZE, WORLD_HEIGHT, CHUNK_SIZE));
        data[i] = get_block_rotation(x as i64 + start_x, y as i64, z as i64 + start_z);
    }
    info!(
        "took {} seconds to generate chunk",
        start.elapsed().as_secs_f32()
    );
    data
}

pub struct CPUChunkProvider {
    thread: JoinHandle<()>,
    buffer: Arc<(Mutex<Option<((i64, i64), Box<Chunk>)>>, Condvar)>,
}
impl CPUChunkProvider {
    pub fn new(max_chunks: i32) -> Self {
        if max_chunks < 1 {
            panic!("max_chunks cannot be smaller than 1")
        }
        let buffer = Arc::new((Mutex::new(None), Condvar::new()));
        let buffer_copy = buffer.clone();
        let thread = thread::spawn(move || {
            for (start_x, start_z) in (0..max_chunks).map(spiral) {
                let value = (
                    (
                        start_x as i64 * (CHUNK_SIZE - CHUNK_MARGIN) as i64,
                        start_z as i64 * (CHUNK_SIZE - CHUNK_MARGIN) as i64,
                    ),
                    /*
                    match Self::get_cache(start_x, start_z) {
                        Some(chunk) => chunk,
                        None => {
                            let chunk = generate_grid(
                                start_x * CHUNK_SIZE as i64,
                                start_z * CHUNK_SIZE as i64,
                                );
                                Self::write_cache(start_x, start_z, chunk.as_ref());
                                chunk
                                }
                            },
                    */
                    generate_grid(
                        start_x as i64 * CHUNK_SIZE as i64,
                        start_z as i64 * CHUNK_SIZE as i64,
                    ),
                );
                let mut lock = buffer_copy.0.lock().unwrap();
                if lock.is_some() {
                    lock = buffer_copy.1.wait(lock).unwrap();
                }
                *lock = Some(value);
                info!("sent chunk");
                buffer_copy.1.notify_one();
            }
        });
        buffer.1.notify_one();
        CPUChunkProvider { thread, buffer }
    }
    pub fn try_next(&mut self) -> Option<((i64, i64), Box<Chunk>)> {
        let mut lock = self.buffer.0.lock().unwrap();
        if !lock.is_some() {
            None
        } else {
            let mut value = None;
            std::mem::swap(&mut value, &mut lock);
            self.buffer.1.notify_one();
            value
        }
    }
    pub fn is_finished(&self) -> bool {
        self.thread.is_finished()
    }
}

impl Iterator for CPUChunkProvider {
    type Item = ((i64, i64), Box<Chunk>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.thread.is_finished() {
            None
        } else {
            let mut lock = self.buffer.0.lock().unwrap();
            if !lock.is_some() {
                info!("waiting on chunk...");
                lock = self.buffer.1.wait(lock).unwrap();
            }
            let mut value = None;
            std::mem::swap(&mut value, &mut lock);
            self.buffer.1.notify_one();
            value
        }
    }
}
