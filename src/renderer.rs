use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec4};
use rusty_spine::{BlendMode, controller::SkeletonCombinedRenderable};

pub mod wgpu;

pub struct RendererCallback {
    pub meshes: Meshes,
    pub scene_view: Mat4,
    pub premultiplied_alpha: bool,
}

pub struct Meshes(pub Vec<SkeletonCombinedRenderable>);

// FIXME(Unavailable): Why this is safe again?
unsafe impl Send for Meshes {}
unsafe impl Sync for Meshes {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct Vertex {
    position: Vec2,
    uv: Vec2,
    color: Vec4,
    dark_color: Vec4,
}

impl Vertex {
    pub fn wgpu_buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        use wgpu::*;

        const ATTRIBUTES: &[VertexAttribute] =
            &vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4];

        VertexBufferLayout {
            array_stride: size_of::<Vertex>() as u64,
            step_mode: VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}

struct SpineBlendMode(BlendMode);

impl SpineBlendMode {
    fn into_blend_state(self, premultiplied_alpha: bool) -> wgpu::BlendState {
        use wgpu::*;
        match self.0 {
            BlendMode::Additive => match premultiplied_alpha {
                // Case 1: Additive Blend Mode, Normal Alpha
                false => BlendState {
                    alpha: BlendComponent {
                        operation: BlendOperation::Add,
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::One,
                    },
                    color: BlendComponent {
                        operation: BlendOperation::Add,
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::One,
                    },
                },
                // Case 2: Additive Blend Mode, Premultiplied Alpha
                true => BlendState {
                    alpha: BlendComponent {
                        operation: BlendOperation::Add,
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::One,
                    },
                    color: BlendComponent {
                        operation: BlendOperation::Add,
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::One,
                    },
                },
            },
            BlendMode::Multiply => match premultiplied_alpha {
                // Case 3: Multiply Blend Mode, Normal Alpha
                false => BlendState {
                    alpha: BlendComponent {
                        operation: BlendOperation::Add,
                        src_factor: BlendFactor::OneMinusSrcAlpha,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                    },
                    color: BlendComponent {
                        operation: BlendOperation::Add,
                        src_factor: BlendFactor::Dst,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                    },
                },
                // Case 4: Multiply Blend Mode, Premultiplied Alpha
                true => BlendState {
                    alpha: BlendComponent {
                        operation: BlendOperation::Add,
                        src_factor: BlendFactor::OneMinusSrcAlpha,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                    },
                    color: BlendComponent {
                        operation: BlendOperation::Add,
                        src_factor: BlendFactor::Dst,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                    },
                },
            },
            BlendMode::Normal => match premultiplied_alpha {
                // Case 5: Normal Blend Mode, Normal Alpha
                false => BlendState {
                    alpha: BlendComponent {
                        operation: BlendOperation::Add,
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                    },
                    color: BlendComponent {
                        operation: BlendOperation::Add,
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                    },
                },
                // Case 6: Normal Blend Mode, Premultiplied Alpha
                true => BlendState {
                    alpha: BlendComponent {
                        operation: BlendOperation::Add,
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                    },
                    color: BlendComponent {
                        operation: BlendOperation::Add,
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                    },
                },
            },
            BlendMode::Screen => match premultiplied_alpha {
                // Case 7: Screen Blend Mode, Normal Alpha
                false => BlendState {
                    alpha: BlendComponent {
                        operation: BlendOperation::Add,
                        src_factor: BlendFactor::OneMinusSrc,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                    },
                    color: BlendComponent {
                        operation: BlendOperation::Add,
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                    },
                },
                // Case 8: Screen Blend Mode, Premultiplied Alpha
                true => BlendState {
                    alpha: BlendComponent {
                        operation: BlendOperation::Add,
                        src_factor: BlendFactor::OneMinusSrc,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                    },
                    color: BlendComponent {
                        operation: BlendOperation::Add,
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                    },
                },
            },
        }
    }
}
