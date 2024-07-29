pub mod chunk;
pub mod plugin;

pub mod util;

// Bits: aaaabbbb
// a: MaxRotation,
// b: Rotation
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct Rotation(pub u8);

impl Rotation {
    #[inline]
    pub fn new(rotation: u8, max_rotation: u8) -> Self {
        Self((max_rotation << 4) + rotation)
    }
    #[inline]
    pub fn get_rotation(self) -> u8 {
        self.0 & 0x0F
    }
    #[inline]
    pub fn get_max_rotation(self) -> u8 {
        (self.0 >> 4) & 0x0F
    }
    #[inline]
    pub fn rotate(self, rotation: u8) -> Self {
        Self::new(
            (self.get_rotation() + rotation) % self.get_max_rotation().max(1),
            self.get_max_rotation(),
        )
    }
}