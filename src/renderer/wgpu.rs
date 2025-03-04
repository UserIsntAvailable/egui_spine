use super::{RendererCallback, SpineBlendMode, Vertex};
use crate::Scene;
use egui::{PaintCallbackInfo, Vec2};
use egui_wgpu::wgpu::util::*;
use egui_wgpu::*;
use glam::{Mat4, Vec4};
use rusty_spine::atlas::{AtlasFilter, AtlasWrap};
use std::{num::NonZero, ops::Deref};
use texture::{ColorProfile, Texture};

pub(super) use egui_wgpu::wgpu::*;

mod texture;

pub struct WgpuContexOptions {}

struct WgpuResources {
    device: Device,
    queue: Queue,
    surface_format: TextureFormat,
    shader: ShaderModule,
    scene_buffer: Buffer,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    texture_bind_group_layout: BindGroupLayout,
    scene_bind_group: BindGroup,
    pipeline_layout: PipelineLayout,
}

pub fn init_wgpu_spine_context(render_state: &RenderState, _options: WgpuContexOptions) {
    set_spine_callbacks();

    let RenderState {
        device,
        queue,
        target_format,
        ..
    } = render_state;

    let shader = device.create_shader_module(include_wgsl!("spine.wgsl"));

    let scene_view = Scene::default().create_scene_view(Vec2::ZERO);
    let scene_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Scene Buffer"),
        contents: bytemuck::bytes_of(&scene_view),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let vertex_buffer = device.create_buffer(&BufferDescriptor {
        label: Some("Spine Vertex Buffer"),
        // TODO(Unavailable): Configuration
        size: 2u64.pow(13) * size_of::<Vertex>() as u64,
        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let index_buffer = device.create_buffer(&BufferDescriptor {
        label: Some("Spine Index Buffer"),
        // TODO(Unavailable): Configuration
        size: 2u64.pow(13) * size_of::<u16>() as u64,
        usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let scene_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Spine Bind Group Layout"),
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                // PERF(Unavailable): Investigate if this actually matters.
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let scene_bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: Some("Spine Scene Bind Group"),
        layout: &scene_bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: scene_buffer.as_entire_binding(),
        }],
    });

    let texture_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Spine Texture Bind Group Layout"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Spine Pipeline Layout"),
        bind_group_layouts: &[&scene_bind_group_layout, &texture_bind_group_layout],
        push_constant_ranges: &[],
    });

    let resources = WgpuResources {
        device: device.clone(),
        queue: queue.clone(),
        surface_format: *target_format,
        shader,
        scene_buffer,
        vertex_buffer,
        index_buffer,
        texture_bind_group_layout,
        scene_bind_group,
        pipeline_layout,
    };
    render_state
        .renderer
        .write()
        .callback_resources
        .insert(resources);
}

impl CallbackTrait for RendererCallback {
    fn prepare(
        &self,
        _: &Device,
        queue: &Queue,
        _: &ScreenDescriptor,
        _: &mut CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<CommandBuffer> {
        let resources: &WgpuResources = resources.get().unwrap();
        if let Some(mut view) =
            queue.write_buffer_with(&resources.scene_buffer, 0, nonzero(size_of::<Mat4>()))
        {
            view.copy_from_slice(bytemuck::bytes_of(&self.scene_view));
        }

        vec![]
    }

    fn paint(
        &self,
        _: PaintCallbackInfo,
        render_pass: &mut RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        let resources: &WgpuResources = resources.get().unwrap();
        let WgpuResources {
            device,
            queue,
            surface_format,
            vertex_buffer,
            index_buffer,
            texture_bind_group_layout,
            scene_bind_group,
            ..
        } = &resources;

        render_pass.set_bind_group(0, scene_bind_group, &[]);
        for mesh in self.meshes.deref() {
            let blend_mode = SpineBlendMode(mesh.blend_mode);
            let blend_state = blend_mode.into_blend_state(self.premultiplied_alpha);
            render_pass.set_pipeline(&resources.create_render_pipeline(blend_state));

            let mut vertices = vec![];
            for vertex_index in 0..mesh.vertices.len() {
                vertices.push(Vertex {
                    position: glam::Vec2 {
                        x: mesh.vertices[vertex_index][0],
                        y: mesh.vertices[vertex_index][1],
                    },
                    uv: glam::Vec2 {
                        x: mesh.uvs[vertex_index][0],
                        y: mesh.uvs[vertex_index][1],
                    },
                    color: Vec4::from_array(mesh.colors[vertex_index]),
                    dark_color: Vec4::from_array(mesh.dark_colors[vertex_index]),
                });
            }

            if vertices.is_empty() {
                continue;
            }

            if let Some(mut view) = queue.write_buffer_with(
                vertex_buffer,
                0,
                nonzero(vertices.len() * size_of::<Vertex>()),
            ) {
                view.copy_from_slice(bytemuck::cast_slice(&vertices));
            }

            let indices_len = mesh.indices.len();
            // NOTE: We don't need to do this with `vertices`, because `size_of::<Vertex>`
            // is divisible by `COPY_BUFFER_ALIGNMENT`.
            let padded_indices_len = if indices_len % COPY_BUFFER_ALIGNMENT as usize != 0 {
                indices_len + 1
            } else {
                indices_len
            };
            if let Some(mut view) = queue.write_buffer_with(
                index_buffer,
                0,
                nonzero(padded_indices_len * size_of::<u16>()),
            ) {
                view[..indices_len * size_of::<u16>()]
                    .copy_from_slice(bytemuck::cast_slice(&mesh.indices));
            }

            let Some(spine_texture) = mesh.attachment_renderer_object else {
                continue;
            };
            let spine_texture = unsafe { &mut *(spine_texture as *mut SpineTexture) };

            if let SpineTexture::Loading { path, sampler_desc } = spine_texture {
                let color_profile = ColorProfile {
                    surface_format: *surface_format,
                    premultiplied_alpha: self.premultiplied_alpha,
                };
                *spine_texture = SpineTexture::Loaded(
                    Texture::from_path(
                        device,
                        queue,
                        &**path,
                        color_profile,
                        sampler_desc,
                        texture_bind_group_layout,
                    )
                    // FIXME(Unavailable): Any error here should be ignored and logged to the
                    // user.
                    .unwrap(),
                )
            };

            let SpineTexture::Loaded(texture) = &spine_texture else {
                unreachable!()
            };
            render_pass.set_bind_group(1, &texture.bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), IndexFormat::Uint16);
            render_pass.draw_indexed(0..indices_len as u32, 0, 0..1);
        }
    }
}

impl WgpuResources {
    fn create_render_pipeline(&self, blend_state: BlendState) -> RenderPipeline {
        self.device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Spine Render Pipeline"),
                layout: Some(&self.pipeline_layout),
                vertex: VertexState {
                    module: &self.shader,
                    entry_point: None,
                    buffers: &[Vertex::wgpu_buffer_layout()],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: &self.shader,
                    entry_point: None,
                    targets: &[Some(ColorTargetState {
                        format: self.surface_format,
                        blend: Some(blend_state),
                        write_mask: ColorWrites::ALL,
                    })],
                    compilation_options: PipelineCompilationOptions::default(),
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    front_face: FrontFace::Ccw,
                    // FIXME(Unavailable): Pass down from `WgpuSpineCallback`.
                    cull_mode: None,
                    ..Default::default()
                },
                multisample: MultisampleState::default(),
                depth_stencil: None,
                multiview: None,
                cache: None,
            })
    }
}

// Texture

enum SpineTexture {
    Loading {
        path: Box<str>,
        sampler_desc: SamplerDescriptor<'static>,
    },
    Loaded(Texture),
}

fn set_spine_callbacks() {
    rusty_spine::extension::set_create_texture_cb(move |page, path| {
        fn convert_filter(filter: AtlasFilter) -> FilterMode {
            match filter {
                AtlasFilter::Nearest => FilterMode::Nearest,
                AtlasFilter::Linear => FilterMode::Linear,
                // TODO(Unavailable): mips
                // TODO(Unavailable): log
                _filter => FilterMode::Linear,
            }
        }
        fn convert_wrap(wrap: AtlasWrap) -> AddressMode {
            match wrap {
                AtlasWrap::MirroredRepeat => AddressMode::MirrorRepeat,
                AtlasWrap::ClampToEdge => AddressMode::ClampToEdge,
                AtlasWrap::Repeat => AddressMode::Repeat,
                // TODO(Unavailable): log
                _wrap => AddressMode::ClampToEdge,
            }
        }
        page.renderer_object().set(SpineTexture::Loading {
            path: path.to_owned().into_boxed_str(),
            sampler_desc: SamplerDescriptor {
                label: Some("Spine Texture Sampler Descriptor"),
                address_mode_u: convert_wrap(page.u_wrap()),
                address_mode_v: convert_wrap(page.v_wrap()),
                mag_filter: convert_filter(page.mag_filter()),
                min_filter: convert_filter(page.min_filter()),
                ..Default::default()
            },
        });
    });

    rusty_spine::extension::set_dispose_texture_cb(|page| unsafe {
        page.renderer_object().dispose::<SpineTexture>()
    });
}

// Utils

fn nonzero(val: usize) -> NonZero<BufferAddress> {
    NonZero::new(val as BufferAddress).expect("value is not zero")
}
