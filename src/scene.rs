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
    pub screen_width: u32,
    pub screen_height: u32,
    pub eye: Vec3,
}

pub struct SceneIterator<'a> {
    scene: &'a Scene,
    size: usize,
    stopped: u32,
    pub pixel00_loc: Vec3,
    pub pixel_delta_u: Vec3,
    pub pixel_delta_v: Vec3,
}


impl Default for Scene {
    fn default() -> Self {
        Scene {
            screen_width: 2000,
            screen_height: 2000,
            eye: Vec3::new(0f32, 2f32, 1f32)
        }
    }

}

impl Scene {
    pub fn with_eye(eye: Vec3) -> Self {
        let mut scene = Self::default();
        scene.eye = eye;
        return scene;
    }

    pub fn sampling_uniform(&self, device: Arc<wgpu::Device>) -> (wgpu::Buffer, wgpu::Buffer) {
        let self_iter = SceneIterator::new(self, 10000).unwrap();

        let pixel_delta_u_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Uniform metadata buffer"),
                contents: bytemuck::cast_slice(&[self_iter.pixel_delta_u]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let pixel_delta_v_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Uniform metadata buffer"),
                contents: bytemuck::cast_slice(&[self_iter.pixel_delta_v]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        (pixel_delta_u_buffer, pixel_delta_v_buffer)
    }
    
    pub async fn collect_pixels(self: Arc<Self>, filename: String, mut pixels_receiver: tokio::sync::mpsc::Receiver<Vec<Ray>>){
        let mut image = image::RgbImage::new(self.screen_width, self.screen_height);
        let total = self.screen_height * self.screen_width;
        let mut so_far = 0;
        while let Some(pixels) = pixels_receiver.recv().await {
            for pixel in pixels.iter() {
                let screen_x = pixel.screen_x;
                let screen_y = pixel.screen_y;
                let color = pixel.color.as_array();
                image.put_pixel(
                    screen_x, screen_y, image::Rgb([
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
        image.save(&filename).unwrap();
        println!("Zapisano {}", &filename);
    }

    pub fn get_balls_bg(self: Arc<Self>, cp: Arc<wgpu::ComputePipeline>, device: Arc<wgpu::Device>) -> wgpu::Buffer {
        let ball = Ball {
            center: Vec3::new(-0.7, 0.0, -1.5),
            radius: 0.5,
            material: 1,
            _padding: Default::default(),
        };

        let balls: Vec<Ball> = vec![
            Ball {
                center: Vec3::new(0.0, -10.5, -1.0),
                radius: 10.0,
                material: 1,
                _padding: Default::default(),
            }, 
            // Ball { center: Vec3::new(0.5, 0.0, -0.7) , radius: 0.5, material: 0, _padding: Default::default()}, 
            // ball
        ];

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ball's buffer"),
            contents: bytemuck::cast_slice(&balls),
            usage: wgpu::BufferUsages::STORAGE,
        });

        buffer

    }

    pub fn get_triangles_bg(self: Arc<Self>, cp: Arc<wgpu::ComputePipeline>, device: Arc<wgpu::Device>) -> wgpu::Buffer {
        let mut loading_options = tobj::LoadOptions::default();
        loading_options.triangulate = true;

        let model = tobj::load_obj("monkey2.obj", &loading_options).unwrap().0;
        let model = model.get(0).unwrap();

        let mut vertices = Vec::with_capacity(1000);
        let mut triangles = Vec::with_capacity(1000);


        for [x, y, z] in model.mesh.positions.array_chunks() {
            vertices.push(
                Vec3::new(x.clone(), y.clone(), z.clone())
            )
        }

        for [v1, v2, v3] in model.mesh.indices.array_chunks() {
            triangles.push(
                Triangle::new(
                    vertices[v1.clone() as usize], 
                    vertices[v2.clone() as usize],
                    vertices[v3.clone() as usize]
            )
        )
        }

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Buffer with triangles"),
            contents: bytemuck::cast_slice(&triangles),
            usage: wgpu::BufferUsages::STORAGE,
        });

        buffer
    }
}


impl<'a> SceneIterator<'a> {
    pub fn new(scene: &'a Scene, size: usize) -> Result<Self, String> {
        let sqrt = (size as f32).sqrt();

        let vup = Vec3::new(0., 1., 0.);
        let look_from = scene.eye;
        let look_at = Vec3::new(0., 0., 0.);
        let fov = 30.0f32;
        let focal_length = (look_at - look_from).dot(look_at - look_from).sqrt();
        let h = (fov.to_radians() / 2.).tan();

        let viewport_height = 2. * h * focal_length;
        let viewport_width = viewport_height * (scene.screen_width as f32 / scene.screen_height as f32);
        let camera_center = scene.eye;

        let w = (look_from - look_at).normalized();
        let u = vup.cross(w).normalized();
        let v = w.cross(u);

        let viewport_u = viewport_width * u;
        let viewport_v = viewport_height * (-v);

        let pixel_delta_u = viewport_u / (scene.screen_width as f32);
        let pixel_delta_v = viewport_v / (scene.screen_height as f32);

        let viewport_upper_left_corner = camera_center - focal_length * w - viewport_u / 2. - viewport_v / 2.;
        let pixel00_loc = viewport_upper_left_corner;

        if (sqrt - sqrt.round()) == 0f32 {
            Ok(SceneIterator {
                scene, size, pixel00_loc, pixel_delta_u, pixel_delta_v,
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
            let (screen_x, screen_y) = (screen_x as f32, screen_y as f32);

            let pixel_center = self.pixel00_loc + (self.pixel_delta_u * screen_x) + (self.pixel_delta_v * screen_y);
            let ray = Ray::new(
                self.scene.eye.clone(),
                pixel_center - self.scene.eye,
                None,
                screen_x as u32,
                screen_y as u32,
            );
            
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