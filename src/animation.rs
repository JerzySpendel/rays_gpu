use std::sync::Arc;
use ultraviolet::Vec3;

use crate::scene::Scene;


pub struct Animation {
    eye_from: Vec3,
    eye_to: Vec3,
    frames: u32,
    device: Arc<wgpu::Device>,
    pipeline: Arc<wgpu::ComputePipeline>,
    queue: Arc<wgpu::Queue>,
}

impl Animation {
    pub fn new(
        eye_from: Vec3, 
        eye_to: Vec3, 
        frames: u32, 
        device: Arc<wgpu::Device>, 
        pipeline: Arc<wgpu::ComputePipeline>, 
        queue: Arc<wgpu::Queue>) -> Self {
            return Self {
                eye_from,
                eye_to,
                frames,
                device,
                pipeline,
                queue,
            }
    }

    pub fn scene_at(&self, frame_at: u32) -> Scene {
        let df = self.eye_to - self.eye_from;
        let current_eye = self.eye_from + df * (frame_at as f32 / self.frames as f32);
        return Scene::with_eye(current_eye);
    }
}