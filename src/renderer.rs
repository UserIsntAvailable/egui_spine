use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec4};
use rusty_spine::BlendMode;

pub mod wgpu;

pub struct RendererCallback {
    pub meshes: Meshes,
    pub scene_view: Mat4,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
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

#[derive(Clone, Copy, Debug)]
pub struct SpineBlendMode(BlendMode);

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

mod meshes {
    use super::{SpineBlendMode, Vertex};
    use glam::{Vec2, Vec4};
    use rusty_spine::c::c_void;
    use rusty_spine::controller::{SkeletonCombinedRenderable, SkeletonController};
    use std::{cell::Cell, sync::Arc};

    pub struct Meshes {
        inner: Vec<SkeletonCombinedRenderable>,
        _controller: Arc<SkeletonController>,
    }

    impl Meshes {
        /// Create a new `Meshes` iterator.
        pub fn new(
            controller: Arc<SkeletonController>,
            renderables: Vec<SkeletonCombinedRenderable>,
        ) -> Self {
            Self {
                _controller: controller,
                inner: renderables,
            }
        }

        pub fn iter(&self) -> impl Iterator<Item = Mesh> {
            self.inner.iter().map(|renderable| {
                let vertices_len = renderable.vertices.len();
                let mut vertices = Vec::with_capacity(vertices_len);
                for vertex_index in 0..vertices_len {
                    vertices.push(Vertex {
                        position: Vec2::from_array(renderable.vertices[vertex_index]),
                        uv: Vec2::from_array(renderable.uvs[vertex_index]),
                        color: Vec4::from_array(renderable.colors[vertex_index]),
                        dark_color: Vec4::from_array(renderable.dark_colors[vertex_index]),
                    });
                }
                Mesh {
                    vertices,
                    indices: &renderable.indices,
                    blend_mode: SpineBlendMode(renderable.blend_mode),
                    premultiplied_alpha: renderable.premultiplied_alpha,
                    attachment: renderable.attachment_renderer_object,
                    was_attachment_borrowed: Cell::new(false),
                }
            })
        }
    }

    pub struct Mesh<'a> {
        pub vertices: Vec<Vertex>,
        pub indices: &'a [u16],
        pub blend_mode: SpineBlendMode,
        pub premultiplied_alpha: bool,
        attachment: Option<*const c_void>,
        was_attachment_borrowed: Cell<bool>,
    }

    impl Mesh<'_> {
        /// # Panics:
        ///
        /// If called more than once :)
        ///
        /// # Safety:
        ///
        /// `T` is the same type registered in the atlas's [`renderer_object`]
        /// inside [`set_create_texture_cb`].
        ///
        /// [`renderer_object`]: rusty_spine::atlas::AtlasPage::renderer_object
        /// [`set_create_texture_cb`]: rusty_spine::extension::set_create_texture_cb
        pub unsafe fn renderer_object<T>(&self) -> Option<&mut T> {
            if self.was_attachment_borrowed.replace(true) {
                panic!("Whoever is modifying the wgpu module made an oopsie daisy :)");
            };

            let Some(attachment) = self.attachment else {
                // FIXME(Unavailable): In practice this is _never_ `None`. Should
                // probably also panic with the message above :)
                return None;
            };

            // FIXME(Unavailable): The `*const c_void` from the attachment type
            // is actually a `*mut c_void` under the hood, however it is being
            // cast to `const` for some reason, and I should probably ask the
            // author why it is doing it, if one of the examples is encouraging
            // casting the pointer back into a `mut` one.

            // SAFETY: Read the safety comments from this function and the ones
            // below.
            Some(unsafe { &mut *(attachment as *mut T) })
        }
    }

    // SAFETY: It is safe for `Meshes` to implement `Send + Sync` (even if it
    // has interior mutability), since there is only _one_ thread that have
    // mutable access to the raw pointer inside `SkeletonCombinedRenderable`.
    // This is guaranteed by:
    //
    // - The `Arc<SkeletonController>` prevents any _safe_ mutation of the
    // underlying controller. This guarantees that only the GPU callback could
    // potentially mutate the `AtlasPage`'s renderer_object, once a `Meshes`
    // struct is created. Storing an `Arc` also has the advantage that any
    // resources that reference the controller would not be deallocated while
    // the callback is running.
    //
    // - FIXME(Unavailable): Explain `Mesh::renderer_object`
    unsafe impl Send for Meshes {}
    // SAFETY: Read above
    unsafe impl Sync for Meshes {}
}
// NOTE: Prevents submodules from accessing fields.
pub use meshes::Meshes;
