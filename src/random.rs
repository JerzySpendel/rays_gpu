use std::{sync::Arc, io::Read};

use image::GenericImageView;
use rand::{self, Rng, random};
use wgpu;

pub fn get_random_image(width: u32, height: u32) -> image::DynamicImage {
    let mut rng = rand::thread_rng();
    let size = 4 * width * height;
    let mut random_floats = Vec::<u8>::with_capacity(size as usize);
    random_floats.resize(size as usize, 0);
    rng.fill(random_floats.as_mut_slice());

    let mut noise_image = image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_vec(width, height, random_floats).unwrap();
    image::DynamicImage::ImageRgba8(noise_image)
}


pub fn prepare_random_texture(
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    size: (u32, u32)
) -> wgpu::BindGroup {

    let size = wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1
        };

    let random_texture = device.create_texture(&wgpu::TextureDescriptor {
        size: size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some("random_texture"),
        view_formats: &[],
    });

    let texture_bg_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Random texture bg layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture { 
                    sample_type: wgpu::TextureSampleType::Float {
                        filterable: true }, 
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false 
                    },
                count: None,
            },
        ],
    });

    let noise_image = get_random_image(size.width, size.height);

    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &random_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
    }, 
        &noise_image.to_rgba8(), 
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(size.width * 4),
            rows_per_image: Some(size.height),
        }, size
    );

    let texture_view = random_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &texture_bg_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture_view)
            },
            // wgpu::BindGroupEntry {
            //     binding: 1,
            //     resource: wgpu::BindingResource::Sampler(&texture_sampler)
            // }
        ],
        label: Some("noise bind group")
    });

    bind_group

}