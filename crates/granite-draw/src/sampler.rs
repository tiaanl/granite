#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
/// Simplified sampler addressing mode.
pub enum SamplerAddressing {
    /// Clamp texture coordinates to the edge.
    ClampToEdge,
    /// Repeat texture coordinates.
    Repeat,
}

impl From<SamplerAddressing> for wgpu::AddressMode {
    fn from(value: SamplerAddressing) -> Self {
        match value {
            SamplerAddressing::ClampToEdge => wgpu::AddressMode::ClampToEdge,
            SamplerAddressing::Repeat => wgpu::AddressMode::Repeat,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
/// Simplified sampler filtering mode.
pub enum SamplerFiltering {
    /// Linear texture filtering.
    Linear,
    /// Nearest-neighbor texture filtering.
    Nearest,
}

impl From<SamplerFiltering> for wgpu::FilterMode {
    fn from(value: SamplerFiltering) -> Self {
        match value {
            SamplerFiltering::Linear => wgpu::FilterMode::Linear,
            SamplerFiltering::Nearest => wgpu::FilterMode::Nearest,
        }
    }
}
