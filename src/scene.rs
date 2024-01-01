
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use ultraviolet::Vec3;
use wgpu::util::DeviceExt;
use crate::{ray::Ray, utils::Triangle};


#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Ball {
    pub center: Vec3,
    pub radius: f32,
    pub material: u32,
    _padding: [u32; 3],
}


unsafe impl Pod for Ball {}
unsafe impl Zeroable for Ball {}


pub struct Scene {
    lu: Vec3,
    width: f32,
    height: f32,
    screen_width: u32,
    screen_height: u32,
    eye: Vec3,
}

pub struct SceneIterator<'a> {
    scene: &'a Scene,
    size: usize,
    stopped: u32,
}


impl Default for Scene {
    fn default() -> Self {
        Scene {
            lu: Vec3::new(-2f32, 2f32, -1f32),
            width: 4f32,
            height: 4f32,
            screen_width: 5000,
            screen_height: 5000,
            eye: Vec3::new(0f32, 0f32, 0f32)
        }
    }
}

impl Scene {
    pub async fn collect_pixels(self: Arc<Self>, mut pixels_receiver: tokio::sync::mpsc::Receiver<Vec<Ray>>){
        let mut image = image::RgbImage::new(self.screen_width, self.screen_height);
        let total = self.screen_height * self.screen_width;
        let mut so_far = 0;
        while let Some(pixels) = pixels_receiver.recv().await {
            for pixel in pixels.iter() {
                let color = pixel.color.as_array();
                image.put_pixel(
                    pixel.screen_x, pixel.screen_y, image::Rgb([
                        (color[0] * 255f32) as u8,
                        (color[1] * 255f32) as u8,
                        (color[2] * 255f32) as u8,
                    ]));
            }
            so_far += pixels.len();
            if so_far % 1000 == 0{
                // println!("{}", (so_far as f32) / (total as f32));
            }
        }
        image.save("output.jpg").unwrap();
        println!("Zapisano output.jpg");
    }

    pub fn get_balls_bg(self: Arc<Self>, cp: Arc<wgpu::ComputePipeline>, device: Arc<wgpu::Device>) -> wgpu::Buffer {
        let ball = Ball {
            center: Vec3::new(-0.7, 0.0, -1.5),
            radius: 0.5,
            material: 1,
            _padding: Default::default(),
        };
        let balls = vec![Ball {
            center: Vec3::new(0.0, -100.5, -1.0),
            radius: 100.0,
            material: 0,
            _padding: Default::default(),
        }, Ball { center: Vec3::new(0.5, 0.0, -0.7) , radius: 0.5, material: 1, _padding: Default::default()}, ball];

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ball's buffer"),
            contents: bytemuck::cast_slice(&balls),
            usage: wgpu::BufferUsages::STORAGE,
        });

        buffer

    }

    pub fn get_triangles_bg(self: Arc<Self>, cp: Arc<wgpu::ComputePipeline>, device: Arc<wgpu::Device>) -> wgpu::Buffer {
        let triangle = Triangle::new(
            Vec3::new(0.5, 0.5, -1.0),
            Vec3::new(0.25, 0.25, -1.0),
            Vec3::new(0.75, 0.25, -1.0),
        );

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Buffer with triangles"),
            contents: bytemuck::cast_slice(&[triangle]),
            usage: wgpu::BufferUsages::STORAGE,
        });

        buffer
    }
}


impl<'a> SceneIterator<'a> {
    pub fn new(scene: &'a Scene, size: usize) -> Result<Self, String> {
        let sqrt = (size as f32).sqrt();

        if (sqrt - sqrt.round()) == 0f32 {
            Ok(SceneIterator {
                scene, size,
                stopped: 0,
            })
        }
        else {
            Err(String::from("`size` must be a square of some number"))
        }
    }
}

impl<'a> Iterator for SceneIterator<'a> {
    type Item = SceneChunk;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stopped >= self.scene.screen_width * self.scene.screen_height {
            return None
        }

        let mut rays: Vec<Ray> = Vec::with_capacity(self.size);
        for pixel_id in self.stopped..(self.scene.screen_width * self.scene.screen_height) {
            let screen_y = pixel_id.div_floor(self.scene.screen_width);
            let screen_x = pixel_id - self.scene.screen_width * screen_y;
            let u = (screen_x as f32) / (self.scene.screen_width as f32);
            let v = (screen_y as f32) / (self.scene.screen_height as f32);
            let ray = Ray::new(
                self.scene.eye.clone(),
                Vec3::new(self.scene.lu.x + u*self.scene.width, self.scene.lu.y - v*self.scene.height, self.scene.lu.z),
                None,
                screen_x,
                screen_y,
            );
            // println!("{:?}", ray);
            rays.push(ray);
            if rays.len() >= self.size {
                self.stopped = pixel_id + 1;
                return Some(SceneChunk::from_vec(rays, self.size));
            }

        }
        Some(SceneChunk::from_vec(rays, self.size))

    }
}

pub struct SceneChunk {
    data: Vec<Ray>,
    size: usize,
}

impl SceneChunk {
    pub fn from_vec(data: Vec<Ray>, size: usize) -> Self {
        SceneChunk {
            data, size
        }
    }
    pub fn get_dimensions(&self) -> (u32, u32) {
        if self.size == self.data.len() {
            let sqrt = (self.size as f32).sqrt() as u32;
            (sqrt, sqrt)
        } else {
            (self.size as u32, 1)
        }
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl AsRef<Vec<Ray>> for SceneChunk {
    fn as_ref(&self) -> &Vec<Ray> {
        &self.data
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_iterator() {
        let scene = Scene { lu: Default::default(), width: 1., height: 1., screen_width: 10, screen_height: 10, eye: Default::default() };
        let iterator = SceneIterator::new(&scene, 25).unwrap();

        assert_eq!(iterator.into_iter().collect::<Vec<_>>().len(), 4);
    }
}