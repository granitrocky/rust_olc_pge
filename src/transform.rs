use super::{
    math_3d::*,
    math_4d::*,
};

#[derive(Debug, Copy, Clone)]
pub struct Transform4 {
    pub rot: Rotor4,
    pub pos: Vector4,
    pub scale: Vector4,
}

#[derive(Debug, Copy, Clone)]
pub struct Transform3 {
    pub rot: Rotor3,
    pub pos: Vector3,
    pub scale: Vector3,
}

impl Transform3 {
    #[rustfmt::skip]
    pub fn to_transform_matrix(self) -> cgmath::Matrix4<f32> {
        let v0 = (Vector3::new(1.0, 0.0, 0.0) * self.rot) * self.scale.x;
        let v1 = (Vector3::new(0.0, 1.0, 0.0) * self.rot) * self.scale.y;
        let v2 = (Vector3::new(0.0, 0.0, 1.0) * self.rot) * self.scale.z;
        cgmath::Matrix4::new(
            v0.x,v1.x,-v2.x,0.0,
            v0.y,v1.y,-v2.y,0.0,
            v0.z,v1.z,-v2.z,0.0,
            -self.pos.dot(v0),-self.pos.dot(v1),self.pos.dot(v2),1.0,
        )
    }

    pub fn forward(&self) -> Vector3 {
        self.rot * Vector3::forward()
    }
}

impl Default for Transform3 {
    fn default() -> Self {
        Self {
            scale: Vector3::one(),
            pos: Vector3::default(),
            rot: Rotor3::default(),
        }
    }
}

#[rustfmt::skip]
impl From<[[f32; 4]; 4]> for Transform3{
    fn from(m_a: [[f32; 4]; 4]) -> Self {
        let pos = Vector3{
            x: m_a[3][0],
            y: m_a[3][1],
            z: m_a[3][2],
        };
        let scale = Vector3{
            x: m_a[0][0].signum() * (m_a[0][0] * m_a[0][0]
                                     + m_a[0][1] * m_a[0][1]
                                     + m_a[0][2] * m_a[0][2]).sqrt(),

            y: m_a[1][1].signum() * (m_a[1][0] * m_a[1][0]
                                     + m_a[1][1] * m_a[1][1]
                                     + m_a[1][2] * m_a[1][2]).sqrt(),

            z: m_a[2][2].signum() * (m_a[2][0] * m_a[2][0]
                                    + m_a[2][1] * m_a[2][1]
                                    + m_a[2][2] * m_a[2][2]).sqrt(),
        };
        let m_b = cgmath::Matrix3::new(
            (m_a[0][0] / scale.x).round_to(5), (m_a[0][1] / scale.x).round_to(5), (m_a[0][2] / scale.x).round_to(5),
            (m_a[1][0] / scale.y).round_to(5), (m_a[1][1] / scale.y).round_to(5), (m_a[1][2] / scale.y).round_to(5),
            (m_a[2][0] / scale.z).round_to(5), (m_a[2][1] / scale.z).round_to(5), (m_a[2][2] / scale.z).round_to(5)
        );
        let rot: Rotor3 = m_b.into();

        Transform3{
            pos,
            rot,
            scale
        }
    }
}

impl From<(Vector3, Rotor3)> for Transform3 {
    fn from(t: (Vector3, Rotor3)) -> Self {
        Self {
            pos: t.0,
            rot: t.1,
            scale: Vector3::one(),
        }
    }
}
