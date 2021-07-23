use super::math_3d::Rotor3;
use super::transform::Transform3;

#[derive(Copy, Clone, Debug, Default)]
pub struct Camera {
    pub clip_far: f32,
    pub clip_near: f32,
    pub h_w: f32,
    pub h_h: f32,
    pub transform: Transform3,
    pub inv_camera_rot: Rotor3,
    pub aspect: f32,
    pub fov: f32,
    pub mat: RawMat,
}


#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RawMat {
    pub view_proj: [[f32; 4]; 4],
    pub view_inv_proj: [[f32; 4]; 4],
    pub position: [f32; 3],
}

impl Default for RawMat {
    fn default() -> Self {
        Self {
            view_proj: [[0.0; 4]; 4],
            view_inv_proj: [[0.0; 4]; 4],
            position: [0.0; 3],
        }
    }
}

impl Camera {
    pub fn new() -> Self{
        Self{
            clip_far: 500.0,
            clip_near: 1.0,
            fov: 90.0,
            aspect: 4.0 / 3.0,
            ..Default::default()
        }
    }
    pub fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let proj = cgmath::perspective(
            cgmath::Deg(self.fov),
            self.aspect,
            self.clip_near,
            self.clip_far,
        );
        OPENGL_TO_WGPU_MATRIX * proj * self.transform.to_transform_matrix()
    }
    pub fn build_reverse_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        use cgmath::SquareMatrix;
        let proj = cgmath::perspective(
            cgmath::Deg(self.fov),
            self.aspect,
            self.clip_near,
            self.clip_far,
        );
        OPENGL_TO_WGPU_MATRIX * proj.invert().unwrap()
    }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);
