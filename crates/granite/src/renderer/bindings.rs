use super::*;

#[derive(Clone, Copy)]
pub(super) struct DrawBinding {
    pub group: u32,
    pub binding: u32,
    pub resource: DrawBindingResource,
}

#[derive(Clone, Copy)]
pub(super) enum DrawBindingResource {
    Uniform(UniformId),
    Texture {
        texture: TextureId,
        visibility: ShaderVisibility,
    },
    Sampler {
        sampler: SamplerId,
        visibility: ShaderVisibility,
    },
}

impl DrawBinding {
    /// Creates a uniform binding descriptor.
    pub fn uniform(group: u32, binding: u32, uniform: UniformId) -> Self {
        Self {
            group,
            binding,
            resource: DrawBindingResource::Uniform(uniform),
        }
    }

    /// Creates a texture binding descriptor with fragment visibility.
    pub fn texture(group: u32, binding: u32, texture: TextureId) -> Self {
        Self::texture_with_visibility(group, binding, texture, ShaderVisibility::Fragment)
    }

    /// Creates a texture binding descriptor with explicit visibility.
    pub fn texture_with_visibility(
        group: u32,
        binding: u32,
        texture: TextureId,
        visibility: ShaderVisibility,
    ) -> Self {
        Self {
            group,
            binding,
            resource: DrawBindingResource::Texture {
                texture,
                visibility,
            },
        }
    }

    /// Creates a sampler binding descriptor with fragment visibility.
    pub fn sampler(group: u32, binding: u32, sampler: SamplerId) -> Self {
        Self::sampler_with_visibility(group, binding, sampler, ShaderVisibility::Fragment)
    }

    /// Creates a sampler binding descriptor with explicit visibility.
    pub fn sampler_with_visibility(
        group: u32,
        binding: u32,
        sampler: SamplerId,
        visibility: ShaderVisibility,
    ) -> Self {
        Self {
            group,
            binding,
            resource: DrawBindingResource::Sampler {
                sampler,
                visibility,
            },
        }
    }
}
