use std::default::Default;
use ultraviolet::Vec3;
use bytemuck::{self, Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Ray {
    pub orig: Vec3,
    __pad_0: u32,
    pub dir: Vec3,
    __pad_1: u32,
    pub color: Vec3,
    pub screen_x: u32,
    pub screen_y: u32,
    __pad_2: [u32; 3]
}

unsafe impl Pod for Ray {}
unsafe impl Zeroable for Ray {}

impl Default for Ray {
    fn default() -> Self {
        Self {
            orig: Vec3::default(),
            __pad_0: 0,
            dir: Vec3::default(),
            __pad_1: 0,
            color: Vec3::default(),
            screen_x: Default::default(),
            screen_y: Default::default(),
            __pad_2: Default::default(),
        }
    }
}

impl Ray {
    pub fn new(orig: Vec3, dir: Vec3, color: Option<Vec3>, screen_x: u32, screen_y: u32) -> Ray {
        let color = color.unwrap_or(Vec3::default());

        Ray{
            orig, dir, screen_x, screen_y, color, __pad_0: 0, __pad_1: 0, __pad_2: Default::default()
        }
    }
}