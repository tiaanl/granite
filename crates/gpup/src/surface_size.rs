/// The dimensions of a rendering surface, in physical pixels.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct SurfaceSize {
    pub width: u32,
    pub height: u32,
}

impl SurfaceSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Return `[width, height]` for convenience.
    pub const fn to_array(self) -> [u32; 2] {
        [self.width, self.height]
    }
}
