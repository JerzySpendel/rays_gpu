use ultraviolet::Vec3;
use bytemuck::*;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub v1: Vec3,
    _pad0: u32,
    pub v2: Vec3,
    _pad1: u32,
    pub v3: Vec3,
    _pad2: u32,
}


impl Triangle {
    pub fn new(v1: Vec3, v2: Vec3, v3: Vec3) -> Self {
        Self {
            v1, v2, v3,
            _pad0: Default::default(),
            _pad1: Default::default(),
            _pad2: Default::default(),
        }
    }
}

unsafe impl Pod for Triangle {}
unsafe impl Zeroable for Triangle {}