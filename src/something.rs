#[repr(C)]
struct A {
    x: u8,
    z: u8,
    y: f64,
}

fn main() {
    println!("{}", std::mem::align_of::<[f64; 3]>());
}