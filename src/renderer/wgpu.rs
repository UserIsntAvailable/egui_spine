use super::{RendererCallback, Vertex};
use bevy_color::{LinearRgba, Srgba};
use egui_wgpu::wgpu::util::{BufferInitDescriptor, DeviceExt, TextureDataOrder};
use egui_wgpu::{CallbackResources, CallbackTrait, RenderState};
use rusty_spine::atlas::{AtlasFilter, AtlasWrap};
use std::num::NonZero;

pub(super) use egui_wgpu::wgpu::*;

type SamplerDesc = SamplerDescriptor<'static>;

pub struct WgpuContexOptions {}

pub fn init_wgpu_spine_context(render_state: &RenderState, _options: WgpuContexOptions) {
    set_spine_callbacks();

    let RenderState {
        device,
        queue,
        target_format,
        ..
    } = render_state;

    let shader = device.create_shader_module(include_wgsl!("spine.wgsl"));

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
        scene_bind_group_layout,
        texture_bind_group_layout,
        pipeline_layout,
    };
    render_state
        .renderer
        .write()
        .callback_resources
        .insert(resources);
}

struct WgpuResources {
    device: Device,
    queue: Queue,
    surface_format: TextureFormat,
    shader: ShaderModule,
    scene_bind_group_layout: BindGroupLayout,
    texture_bind_group_layout: BindGroupLayout,
    pipeline_layout: PipelineLayout,
}

impl CallbackTrait for RendererCallback {
    fn paint(
        &self,
        _: egui::PaintCallbackInfo,
        render_pass: &mut RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        let resources: &WgpuResources = resources.get().unwrap();
        let WgpuResources {
            device,
            queue,
            scene_bind_group_layout,
            ..
        } = &resources;

        let scene_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Spine Scene Buffer"),
            contents: bytemuck::bytes_of(&self.scene_view),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let scene_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Spine Scene Bind Group"),
            layout: &scene_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: scene_buffer.as_entire_binding(),
            }],
        });
        render_pass.set_bind_group(0, &scene_bind_group, &[]);

        for mesh in self.meshes.iter() {
            if mesh.vertices.is_empty() {
                continue;
            }

            let blend_state = mesh.blend_mode.into_blend_state(mesh.premultiplied_alpha);

            // SAFETY: `WgpuTexture` is the registered type in
            // `set_create_texture_cb`.
            let spine_texture = unsafe { mesh.renderer_object::<WgpuTexture>() };
            let Some(spine_texture) = spine_texture else {
                continue;
            };

            let vertex_buffer_size = (mesh.vertices.len() * size_of::<Vertex>()) as BufferAddress;

            let indices_len = mesh.indices.len();
            // NOTE: We don't need to do this with `mesh.vertices`, because
            // `size_of::<Vertex>` is divisible by `COPY_BUFFER_ALIGNMENT`.
            let padded_index_buffer_size = {
                let len = if indices_len % COPY_BUFFER_ALIGNMENT as usize == 0 {
                    indices_len
                } else {
                    indices_len + 1
                };
                (len * size_of::<u16>()) as BufferAddress
            };

            if let WgpuTexture::Loading { path, sampler_desc } = spine_texture {
                let vertex_buffer = device.create_buffer(&BufferDescriptor {
                    label: Some("Spine Vertex Buffer"),
                    size: vertex_buffer_size,
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                let index_buffer = device.create_buffer(&BufferDescriptor {
                    label: Some("Spine Index Buffer"),
                    size: padded_index_buffer_size,
                    usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                let pipeline = resources.create_render_pipeline(blend_state);
                let texture_bind_group = resources
                    .create_texture_bind_group(path, mesh.premultiplied_alpha, sampler_desc)
                    // FIXME(Unavailable): Any error here should be ignored and
                    // logged to the user.
                    .unwrap();

                *spine_texture = WgpuTexture::Loaded {
                    pipeline,
                    vertex_buffer,
                    index_buffer,
                    texture_bind_group,
                };
            };

            let WgpuTexture::Loaded {
                pipeline,
                vertex_buffer,
                index_buffer,
                texture_bind_group,
            } = &spine_texture
            else {
                unreachable!()
            };

            if let Some(mut view) =
                queue.write_buffer_with(vertex_buffer, 0, nonzero(vertex_buffer_size))
            {
                view.copy_from_slice(bytemuck::cast_slice(&mesh.vertices));
            }
            if let Some(mut view) =
                queue.write_buffer_with(index_buffer, 0, nonzero(padded_index_buffer_size))
            {
                view[..indices_len * size_of::<u16>()]
                    .copy_from_slice(bytemuck::cast_slice(&mesh.indices));
            }

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(1, texture_bind_group, &[]);
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
                    // FIXME(Unavailable): Pass down from `RendererCallback`.
                    cull_mode: None,
                    ..Default::default()
                },
                multisample: MultisampleState::default(),
                depth_stencil: None,
                multiview: None,
                cache: None,
            })
    }

    fn create_texture_bind_group(
        &self,
        path: &str,
        premultiplied_alpha: bool,
        sampler_desc: &SamplerDesc,
    ) -> image::ImageResult<BindGroup> {
        let bytes = std::fs::read(&path)?;
        let image = image::load_from_memory(&bytes)?;

        let pixels = image.to_rgba8();
        let (width, height) = pixels.dimensions();
        let mut pixels = pixels.into_vec();

        // TODO(Unavailable): Rewrite with `epaint`.
        if self.surface_format.is_srgb() && premultiplied_alpha {
            for i in 0..(pixels.len() / 4) {
                let srgba = Srgba::rgba_u8(
                    pixels[i * 4],
                    pixels[i * 4 + 1],
                    pixels[i * 4 + 2],
                    pixels[i * 4 + 3],
                );
                let srgba = if srgba.alpha != 0. {
                    Srgba::new(
                        srgba.red / srgba.alpha,
                        srgba.green / srgba.alpha,
                        srgba.blue / srgba.alpha,
                        srgba.alpha,
                    )
                } else {
                    Srgba::new(0., 0., 0., 0.)
                };
                let mut lrgba = LinearRgba::from(srgba);
                lrgba.red *= lrgba.alpha;
                lrgba.green *= lrgba.alpha;
                lrgba.blue *= lrgba.alpha;
                let srgba = Srgba::from(lrgba);
                pixels[i * 4] = (srgba.red * 255.) as u8;
                pixels[i * 4 + 1] = (srgba.green * 255.) as u8;
                pixels[i * 4 + 2] = (srgba.blue * 255.) as u8;
                pixels[i * 4 + 3] = (srgba.alpha * 255.) as u8;
            }
        }

        let format = if self.surface_format.is_srgb() {
            TextureFormat::Rgba8UnormSrgb
        } else {
            TextureFormat::Rgba8Unorm
        };
        let texture = self.device.create_texture_with_data(
            &self.queue,
            &TextureDescriptor {
                label: Some("Spine Texture"),
                size: Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format,
                usage: TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            TextureDataOrder::default(),
            &pixels,
        );

        let view = texture.create_view(&TextureViewDescriptor {
            label: Some("Spine Texture View"),
            ..Default::default()
        });
        let sampler = self.device.create_sampler(sampler_desc);
        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Spine Texture Bind Group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        Ok(bind_group)
    }
}

// Texture
enum WgpuTexture {
    Loading {
        path: Box<str>,
        sampler_desc: SamplerDesc,
    },
    Loaded {
        pipeline: RenderPipeline,
        vertex_buffer: Buffer,
        index_buffer: Buffer,
        texture_bind_group: BindGroup,
    },
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
        page.renderer_object().set(WgpuTexture::Loading {
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

    rusty_spine::extension::set_dispose_texture_cb(|page|
        // SAFETY: `SpineTexture` is a rust type that only contains values
        // allocated with the rust allocator.
        unsafe { page.renderer_object().dispose::<WgpuTexture>() });
}

fn nonzero(val: BufferAddress) -> NonZero<BufferAddress> {
    NonZero::new(val).expect("value is not zero")
}
