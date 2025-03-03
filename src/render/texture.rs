use anyhow::*;
use bevy_color::{LinearRgba, Srgba};
use egui_wgpu::wgpu::Texture as WgpuTexture;
use egui_wgpu::wgpu::util::{DeviceExt as _, TextureDataOrder};
use egui_wgpu::wgpu::*;
use std::path::Path;

#[derive(Clone)]
pub struct Texture {
    pub texture: WgpuTexture,
    pub bind_group: BindGroup,
}

impl Texture {
    pub fn from_path<P>(
        device: &Device,
        queue: &Queue,
        path: P,
        premultiplied_alpha: bool,
        sampler_desc: &SamplerDescriptor<'static>,
        bind_group_layout: &BindGroupLayout,
    ) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        Self::__from_path(
            device,
            queue,
            path.as_ref(),
            premultiplied_alpha,
            sampler_desc,
            bind_group_layout,
        )
    }

    fn __from_path(
        device: &Device,
        queue: &Queue,
        path: &Path,
        premultiplied_alpha: bool,
        sampler_desc: &SamplerDescriptor<'static>,
        bind_group_layout: &BindGroupLayout,
    ) -> Result<Self> {
        let bytes = std::fs::read(&path)?;
        let image = image::load_from_memory(&bytes)?;

        let pixels = image.to_rgba8();
        let (width, height) = pixels.dimensions();
        let mut pixels = pixels.into_vec();

        // TODO(Unavailable): Rewrite with `epaint`.
        // FIXME(Unavailable): This is only needed if the fragment shader format is Srgb
        if false {
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

        let texture = device.create_texture_with_data(
            queue,
            &TextureDescriptor {
                label: Some("Texture"),
                size: Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            TextureDataOrder::default(),
            &pixels,
        );

        let view = texture.create_view(&TextureViewDescriptor {
            label: Some("Texture View"),
            ..Default::default()
        });

        let sampler = device.create_sampler(sampler_desc);

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Spine Texture Bind Group"),
            layout: bind_group_layout,
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

        Ok(Self {
            texture,
            bind_group,
        })
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}
