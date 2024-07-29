use std::num::Wrapping;

use bevy_meshem::util::{one_d_cords, three_d_cords};

use crate::constants::GRID_SIZE;

use super::Rotation;

pub fn get_block_rotation(x: i64, y: i64, z: i64) -> u8 {
    (get_rotation_from_seed(get_rendering_seed(x, y, z) >> 16).abs() & 3) as u8
}
pub fn get_rendering_seed(x: i64, y: i64, z: i64) -> i64 {
    let (x, y, z) = (Wrapping(x), Wrapping(y), Wrapping(z));
    let l = (x * Wrapping(3129871)) ^ (z * Wrapping(116129781)) ^ y;
    let l = (l * l * Wrapping(42317861)) + l * Wrapping(11);
    l.0
}

fn get_rotation_from_seed(seed: i64) -> i32 {
    let seed = (seed ^ 0x5DEECE66D) & ((1 << 48) - 1);
    let value = (((seed.wrapping_mul(0xBB20B4600A69).wrapping_add(0x40942DE6BA) as u64) >> 16) & (0xFFFFFFFF)) as i64;
    value as i32
}

#[inline]
pub fn check_rotation(desired_rotation: Rotation, rotation: u8) -> bool {
    desired_rotation.get_max_rotation() <= 1
        || rotation % (desired_rotation.get_max_rotation()) == desired_rotation.get_rotation()
}

pub fn rotate_grid(
    grid: &[Rotation ;GRID_SIZE.0 * GRID_SIZE.1 * GRID_SIZE.2],
    rotation: u8,
) -> Box<[Rotation ;GRID_SIZE.0 * GRID_SIZE.1 * GRID_SIZE.2]> {
    let mut new_grid = Box::new(*grid);
    grid.iter().enumerate().map(|(index, value, )| (three_d_cords(index, GRID_SIZE), value))
        .for_each(|(pos, value)| {
            new_grid[one_d_cords(rotate_pos(pos, rotation), GRID_SIZE)] = value.rotate(rotation);
        });
    new_grid
}

pub fn rotate_pos(mut pos: (usize, usize, usize), rotation: u8) -> [usize; 3] {
    for _ in 0..rotation {
        (pos.0, pos.2) = (pos.0, pos.2);
        todo!()
    }
    [pos.0, pos.1, pos.2]
}

pub fn spiral(n: i32) -> (i32, i32) {
    if n == 0 {
        return (0, 0);
    }
    // given n an index in the squared spiral
    // p the sum of point in inner square
    // a the position on the current square
    // n = p + a
    let n = n - 1;

    let r = (((((n + 1) as f64).sqrt() - 1.0) / 2.0).floor() + 1.0) as i32;

    // compute radius : inverse arithmetic sum of 8+16+24+...=
    let p = (8 * r * (r - 1)) / 2;
    // compute total point on radius -1 : arithmetic sum of 8+16+24+...

    let en = r * 2;
    // points by face

    let a = (1 + n - p) % (r * 8);
    // compute the position and shift it so the first is (-r,-r) but (-r+1,-r)
    // so square can connect

    match (a as f64 / (r * 2) as f64).floor() as i64 {
        // find the face : 0 top, 1 right, 2, bottom, 3 left
        0 => (a - r, -r),
        1 => (r, (a % en) - r),
        2 => (r - (a % en), r),
        3 => (-r, r - (a % en)),
        _ => unreachable!(),
    }
}

pub fn check_rotation2(desired_rotation: Rotation, rotation: u8) -> bool {
    (desired_rotation.0 >> 4) & 0x0F <= 1
        || rotation % ((desired_rotation.0 >> 4) & 0x0F) == desired_rotation.0 & 0x0F
}