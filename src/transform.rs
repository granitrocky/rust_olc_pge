use crate::math_3d::*;
use crate::math_4d::*;

#[derive(Debug, Copy, Clone)]
pub struct Transform4 {
    pub rot: Rotor4,
    pub pos: Vector4,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct Transform3 {
    pub rot: Rotor3,
    pub pos: Vector3,
}

impl Transform3 {
    pub fn to_transform_matrix(self) -> cgmath::Matrix4<f32> {
        let v0 = Vector3::new(1.0, 0.0, 0.0) * self.rot;
        let v1 = Vector3::new(0.0, 1.0, 0.0) * self.rot;
        let v2 = Vector3::new(0.0, 0.0, 1.0) * self.rot;

        cgmath::Matrix4::new(
            v0.x,
            v1.x,
            -v2.x,
            0.0,
            v0.y,
            v1.y,
            -v2.y,
            0.0,
            v0.z,
            v1.z,
            -v2.z,
            0.0,
            -self.pos.dot(&v0),
            -self.pos.dot(&v1),
            self.pos.dot(&v2),
            1.0,
        )
    }
}

impl From<(Vector3, Rotor3)> for Transform3 {
    fn from(t: (Vector3, Rotor3)) -> Self {
        Self { pos: t.0, rot: t.1 }
    }
}
