#![feature(int_roundings)]

mod ray;
mod utils;
mod scene;
mod random;

use std::borrow::Cow;
use wgpu::{self, ComputePipeline};
use tokio;
use bytemuck;
use wgpu::util::DeviceExt;
use ray::Ray;
use scene::{Scene, SceneIterator};

use std::sync::Arc;

use crate::random::prepare_random_texture;



#[tokio::main]
async fn main() -> Result<(), String> {
    let instance = wgpu::Instance::default();
    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
    }).await.unwrap();
    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
        label: None,
        features: wgpu::Features::default(),
        limits: wgpu::Limits::default(),
    }, None).await.unwrap();
    let device = Arc::new(device);
    let queue = Arc::new(queue);
    println!("{:?}", adapter.get_info());

    let cs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl")))
    });

    let compute_pipeline = Arc::new(device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &cs_module,
        entry_point: "main",
    }));

    let scene = Arc::new(Scene::default());
    let (pixels_stream_sender, pixels_stream_receiver) = tokio::sync::mpsc::channel(10);
    let (ray_sender, ray_receiver) = tokio::sync::mpsc::channel(10);
    tokio::spawn(pixel_sender(pixels_stream_sender, scene.clone()));
    tokio::spawn(compute_pixels(pixels_stream_receiver, ray_sender, device.clone(), compute_pipeline.clone(), queue.clone(), scene.clone()));

    tokio::spawn(scene.collect_pixels(ray_receiver)).await;
    Ok(())
}

async fn pixel_sender(pixel_stream: tokio::sync::mpsc::Sender<Vec<Ray>>, scene: Arc<Scene>){
    for chunk in SceneIterator::new(&scene, 50000) {
        pixel_stream.send(chunk).await;
    }

}

async fn compute_pixels(
    mut pixel_stream: tokio::sync::mpsc::Receiver<Vec<Ray>>,
    pixels_stream_out: tokio::sync::mpsc::Sender<Vec<Ray>>,
    device: Arc<wgpu::Device>,
    cp: Arc<ComputePipeline>,
    queue: Arc<wgpu::Queue>,
    scene: Arc<Scene> ) {
    while let Some(pixel) = pixel_stream.recv().await {
        let scene = scene.clone();
        let t = std::time::Instant::now();
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging buffer"),
            size: (std::mem::size_of::<Ray>() * pixel.len()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let storage_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Storage buffer"),
            contents: bytemuck::cast_slice(&pixel),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC
        });

        let balls_buffer = scene.get_balls_bg(cp.clone(), device.clone());
        let bind_group_layout: wgpu::BindGroupLayout = cp.get_bind_group_layout(0);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: storage_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: balls_buffer.as_entire_binding(),
                }]
        });

        let texture_side_size = (pixel.len() as f32).sqrt().ceil() as u32;
        let noise_bg = prepare_random_texture(device.clone(), queue.clone(), (texture_side_size, texture_side_size));


        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        cpass.set_pipeline(&cp);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.set_bind_group(1, &noise_bg, &[]);
        cpass.dispatch_workgroups(pixel.len() as u32, 1, 1);
        drop(cpass);

        encoder.copy_buffer_to_buffer(&storage_buffer, 0, &staging_buffer, 0, (std::mem::size_of::<Ray>() * pixel.len()) as wgpu::BufferAddress);
        queue.submit(Some(encoder.finish()));

        let (sender, receiver) = tokio::sync::oneshot::channel();
        let the_slice = staging_buffer.slice(..);
        the_slice.map_async(wgpu::MapMode::Read, move |v| {
            sender.send(v).unwrap();
        });

        device.poll(wgpu::Maintain::Wait);
        if let Ok(()) = receiver.await.unwrap() {
            let pixels = {
                let data = the_slice.get_mapped_range();
                let data = bytemuck::cast_slice::<u8, Ray>(&data);
                data.to_vec()
            };
            pixels_stream_out.send(pixels).await;
        }

    }
    println!("Closing the channel");
}