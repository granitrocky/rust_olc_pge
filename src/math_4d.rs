#![allow(non_snake_case)]
use crate::math_3d::Rotor3;
use std::cmp::Ordering;
use std::ops::Add;

#[derive(Debug, Copy, Clone, Default)]
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[derive(Default)]
pub struct BiVector4 {
    pub b12: f32,
    pub b13: f32,
    pub b14: f32,
    pub b23: f32,
    pub b24: f32,
    pub b34: f32,
    pub b1234: f32,
}

#[derive(Debug, Copy, Clone)]
pub struct Rotor4 {
    pub a: f32,
    pub b12: f32,
    pub b13: f32,
    pub b14: f32,
    pub b23: f32,
    pub b24: f32,
    pub b34: f32,
    pub b1234: f32,
}

impl Default for Rotor4 {
    fn default() -> Rotor4 {
        (1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0).into()
    }
}

impl From<(f32, f32, f32, f32, f32, f32, f32)> for BiVector4 {
    fn from(tuple: (f32, f32, f32, f32, f32, f32, f32)) -> Self {
        BiVector4 {
            b12: tuple.0,
            b13: tuple.1,
            b14: tuple.2,
            b23: tuple.3,
            b24: tuple.4,
            b34: tuple.5,
            b1234: tuple.6,
        }
    }
}
impl From<(Vector4, Vector4)> for BiVector4 {
    fn from(tuple: (Vector4, Vector4)) -> Self {
        outer(&tuple.0, &tuple.1)
    }
}
impl From<(f32, f32, f32, f32)> for Vector4 {
    fn from(tuple: (f32, f32, f32, f32)) -> Self {
        Vector4 {
            x: tuple.0,
            y: tuple.1,
            z: tuple.2,
            w: tuple.3,
        }
    }
}
impl From<(f32, f32, f32, f32, f32, f32, f32, f32)> for Rotor4 {
    fn from(tuple: (f32, f32, f32, f32, f32, f32, f32, f32)) -> Self {
        Rotor4 {
            a: tuple.0,
            b12: tuple.1,
            b13: tuple.2,
            b14: tuple.3,
            b23: tuple.4,
            b24: tuple.5,
            b34: tuple.6,
            b1234: tuple.7,
        }
    }
}

pub fn outer(u: &Vector4, v: &Vector4) -> BiVector4 {
    let bv1: BiVector4 = (
        u.x * v.y - u.y * v.x,
        u.x * v.z - u.z * v.x,
        u.x * v.w - u.w * v.x,
        u.y * v.z - u.z * v.y,
        u.y * v.w - u.w * v.y,
        u.z * v.w - u.w * v.z,
        0.0,
    )
        .into();
    (
        bv1.b12,
        bv1.b13,
        bv1.b14,
        bv1.b23,
        bv1.b24,
        bv1.b34,
        bv1.b12 * bv1.b34 + bv1.b14 * bv1.b23 + bv1.b34 * bv1.b12 + bv1.b23 * bv1.b14
            - bv1.b24 * bv1.b13
            - bv1.b13 * bv1.b24,
    )
        .into()
}

impl Rotor4 {
    pub fn from_bivector(bv: &BiVector4) -> Self {
        let mut r = Self {
            a: 1.0 - bv.length(),
            b12: bv.b12,
            b13: bv.b13,
            b14: bv.b14,
            b23: bv.b23,
            b24: bv.b24,
            b34: bv.b34,
            b1234: bv.b1234,
        };
        r.normalize();
        r
    }

    pub fn from_vectors(v_from: &Vector4, v_to: &Vector4) -> Self {
        //This 1 + dot product will give you half the angle
        let a = 1.0 + v_to.dot(v_from);
        let minus_b = outer(v_to, v_from);
        let mut r = Rotor4 {
            a,
            b12: minus_b.b12,
            b13: minus_b.b13,
            b14: minus_b.b14,
            b23: minus_b.b23,
            b24: minus_b.b24,
            b34: minus_b.b34,
            b1234: minus_b.b1234,
        };
        r.normalize();
        r
    }

    pub fn from_angle_and_axis(angleRadian: f32, bvPlane: &mut BiVector4) -> Self {
        let angle_half = angleRadian / 2.0;
        bvPlane.normalize();
        let sina = angle_half.sin();
        Rotor4 {
            a: angle_half.cos(),
            b12: -sina * bvPlane.b12,
            b13: -sina * bvPlane.b13,
            b14: -sina * bvPlane.b14,
            b23: -sina * bvPlane.b23,
            b24: -sina * bvPlane.b24,
            b34: -sina * bvPlane.b34,
            b1234: -sina * bvPlane.b1234,
        }
    }

    pub fn rotate_vector(&self, v: &Vector4) -> Vector4 {
        let q321: f32 = v.x * self.b23 - v.y * self.b13 + v.z * self.b12 - v.w * self.b1234;
        let q124: f32 = -v.x * self.b24 + v.y * self.b14 - v.z * self.b1234 - v.w * self.b12;
        let q431: f32 = v.x * self.b34 - v.y * self.b1234 - v.z * self.b14 + v.w * self.b13;
        let q234: f32 = -v.x * self.b1234 - v.y * self.b34 + v.z * self.b24 - v.w * self.b23;

        let q = Vector4 {
            x: self.a * v.x - v.y * self.b12 - v.z * self.b13 - v.w * self.b14,
            y: self.a * v.y + v.x * self.b12 - v.z * self.b23 + v.w * self.b24,
            z: self.a * v.z + v.x * self.b13 + v.y * self.b23 - v.w * self.b34,
            w: self.a * v.w + v.x * self.b14 + v.y * self.b24 + v.z * self.b34,
        };

        Vector4 {
            x: self.a * q.x - q.y * self.b12 - q.z * self.b13 + q321 * self.b23
                - q.w * self.b14
                - q124 * self.b24
                + q431 * self.b34
                + q234 * self.b1234,

            y: self.a * q.y + q.x * self.b12 - q321 * self.b13 - q.z * self.b23 + q124 * self.b14
                - q.w * self.b24
                - q234 * self.b34
                + q431 * self.b1234,

            z: self.a * q.z + q321 * self.b12 + q.x * self.b13 + q.y * self.b23 - q431 * self.b14
                + q234 * self.b24
                - q.w * self.b34
                + q124 * self.b1234,

            w: self.a * q.w - q124 * self.b12 + q431 * self.b13 - q234 * self.b23
                + q.x * self.b14
                + q.y * self.b24
                + q.z * self.b34
                + q321 * self.b1234,
        }
    }
    /*    pub fn rotate_by_rotor(self, r: Rotor4) -> Rotor4{
        let rev = self.reverse();
        self * r * rev
    }*/
    pub fn reverse(&self) -> Rotor4 {
        Rotor4 {
            a: self.a,
            b12: -self.b12,
            b13: -self.b13,
            b14: -self.b14,
            b23: -self.b23,
            b24: -self.b24,
            b34: -self.b34,
            b1234: -self.b1234,
        }
    }
    pub fn length_sqrd(&self) -> f32 {
        self.a * self.a
            + self.b12 * self.b12
            + self.b13 * self.b13
            + self.b14 * self.b14
            + self.b23 * self.b23
            + self.b24 * self.b24
            + self.b34 * self.b34
    }

    pub fn length(&self) -> f32 {
        self.length_sqrd().sqrt()
    }

    pub fn normalize(&mut self) {
        let n = self.normal();
        self.a = n.a;
        self.b12 = n.b12;
        self.b13 = n.b13;
        self.b14 = n.b14;
        self.b23 = n.b23;
        self.b24 = n.b24;
        self.b34 = n.b34;
        self.b1234 = n.b1234;
    }
    pub fn normal(&self) -> Rotor4 {
        let l = self.length();
        Rotor4 {
            a: self.a / l,
            b12: self.b12 / l,
            b13: self.b13 / l,
            b14: self.b14 / l,
            b23: self.b23 / l,
            b24: self.b24 / l,
            b34: self.b34 / l,
            b1234: self.b1234 / l,
        }
    }
    //TODO: Implement this
    /*
    // convert to matrix
    // non-optimized
        inline Matrix3 Rotor3::toMatrix3() const
        {
        Vector3 v0 = rotate( Vector3(1,0,0) );
        Vector3 v1 = rotate( Vector3(0,1,0) );
        Vector3 v2 = rotate( Vector3(0,0,1) );
        return Matrix3( v0, v1, v2 );
    }
    */

    // geometric product (for reference), produces twice the angle, negative direction
    //pub fn geo( a: &Vector4, b: &Vector4 ) -> Rotor4
    /*{
        Rotor4::from_bivector( a.dot(b), &outer(&a,&b) )
    }*/
}

impl BiVector4 {
    pub fn normalize(&mut self) {
        let l = self.length();
        self.b12 /= l;
        self.b13 /= l;
        self.b14 /= l;
        self.b23 /= l;
        self.b24 /= l;
        self.b34 /= l;
        self.b1234 /= l;
    }
    pub fn length(&self) -> f32 {
        self.length_sqrd().sqrt()
    }

    pub fn length_sqrd(&self) -> f32 {
        self.b12 * self.b12
            + self.b13 * self.b13
            + self.b14 * self.b14
            + self.b23 * self.b23
            + self.b24 * self.b24
            + self.b34 * self.b34
    }
}

impl std::ops::Mul for Rotor4 {
    type Output = Rotor4;

    fn mul(self, rhs: Self) -> Self::Output {
        Rotor4 {
            a: self.a * rhs.a
                - self.b12 * rhs.b12
                - self.b13 * rhs.b13
                - -self.b14 * rhs.b14
                - self.b23 * rhs.b23
                - self.b24 * rhs.b24
                - self.b34 * rhs.b34
                + self.b1234 * rhs.b1234,

            b12: self.a * rhs.b12 + self.b12 * rhs.a - self.b13 * rhs.b23 - self.b14 * rhs.b24
                + self.b23 * rhs.b13
                + self.b24 * rhs.b14
                - self.b34 * rhs.b1234
                - self.b1234 * rhs.b34,

            b13: self.a * rhs.b13 + self.b12 * rhs.b23 + self.b13 * rhs.a
                - self.b14 * rhs.b34
                - self.b23 * rhs.b12
                + self.b24 * rhs.b1234
                + self.b34 * rhs.b14
                + self.b1234 * rhs.b24,

            b14: self.a * rhs.b14 + self.b12 * rhs.b24 + self.b13 * rhs.b34 + self.b14 * rhs.a
                - self.b23 * rhs.b1234
                - self.b24 * rhs.b12
                - self.b34 * rhs.b13
                - self.b1234 * rhs.b23,

            b23: self.a * rhs.b23 - self.b12 * rhs.b13 + self.b13 * rhs.b12 - self.b14 * rhs.b1234
                + self.b23 * rhs.a
                - self.b24 * rhs.b34
                + self.b34 * rhs.b24
                - self.b1234 * rhs.b14,

            b24: self.a * rhs.b24 - self.b12 * rhs.b14
                + self.b13 * rhs.b1234
                + self.b14 * rhs.b12
                + self.b23 * rhs.b34
                + self.b24 * rhs.a
                - self.b34 * rhs.b23
                + self.b1234 * rhs.b13,

            b34: self.a * rhs.b34 - self.b12 * rhs.b1234 - self.b13 * rhs.b14 + self.b14 * rhs.b13
                - self.b23 * rhs.b24
                + self.b24 * rhs.b23
                + self.b34 * rhs.a
                - self.b1234 * rhs.b12,

            b1234: self.a * rhs.b1234 + self.b12 * rhs.b34 - self.b13 * rhs.b24
                + self.b14 * rhs.b23
                + self.b23 * rhs.b14
                - self.b24 * rhs.b13
                + self.b34 * rhs.b12
                + self.b1234 * rhs.a,
        }
    }
}
impl std::ops::MulAssign for Rotor4 {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Vector4 {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Vector4 {
        Vector4 { x, y, z, w }
    }
    pub fn dot(&self, rhs: &Vector4) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z + self.w * rhs.w
    }

    pub fn normal(&self) -> Vector4 {
        *self / self.length()
    }
    pub fn length(&self) -> f32 {
        self.length_sqrd().sqrt()
    }
    pub fn length_sqrd(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w
    }
}

impl std::ops::Mul<Vector4> for Rotor4 {
    type Output = Vector4;

    fn mul(self, rhs: Vector4) -> Self::Output {
        self.rotate_vector(&rhs)
    }
}
impl std::ops::Mul<f32> for Rotor4 {
    type Output = Rotor4;

    fn mul(self, rhs: f32) -> Self::Output {
        let bv: BiVector4 = (
            self.b12 * rhs,
            self.b13 * rhs,
            self.b14 * rhs,
            self.b23 * rhs,
            self.b24 * rhs,
            self.b34 * rhs,
            self.b1234 * rhs,
        )
            .into();
        Rotor4::from_bivector(&bv)
    }
}

impl std::ops::Mul<Rotor4> for Vector4 {
    type Output = Rotor4;

    fn mul(self, rhs: Rotor4) -> Self::Output {
        let v2 = rhs * self;
        Rotor4::from_vectors(&self, &v2)
    }
}
/*
pub struct Mat4x4{
    pub m: [[f32;4]; 4],
}

impl Mat4x4{
    pub fn multiply_vector(&self, v: &Vector3) -> Vector3{
        let w =
            v.x * self.m[0][3] + v.y * self.m[1][3] + v.z * self.m[2][3] + self.m[3][3];
        let mut r_v: Vector3 = (
            v.x * self.m[0][0] + v.y * self.m[1][0] + v.z * self.m[2][0] + self.m[3][0],
            v.x * self.m[0][1] + v.y * self.m[1][1] + v.z * self.m[2][1] + self.m[3][1],
            v.x * self.m[0][2] + v.y * self.m[1][2] + v.z * self.m[2][2] + self.m[3][2],
            ).into();
        if w != 0.0{
            r_v /= w;
        }
        r_v
    }
}

impl From<[[f32;4];4]> for Mat4x4{
    fn from(m: [[f32; 4]; 4]) -> Self {
        Self{
            m
        }
    }
}
*/

impl std::ops::Add for Vector4 {
    type Output = Vector4;

    fn add(self, rhs: Self) -> Self::Output {
        Self::Output {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
            w: self.w + rhs.w,
        }
    }
}

impl std::ops::Sub for Vector4 {
    type Output = Vector4;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::Output {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
            w: self.w - rhs.w,
        }
    }
}

impl std::ops::Div for Vector4 {
    type Output = Vector4;

    fn div(self, rhs: Self) -> Self::Output {
        Self::Output {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
            z: self.z / rhs.z,
            w: self.w / rhs.w,
        }
    }
}

impl std::ops::Div<f32> for Vector4 {
    type Output = Vector4;

    fn div(self, rhs: f32) -> Self::Output {
        Self::Output {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
            w: self.w / rhs,
        }
    }
}
impl std::ops::Mul<f32> for Vector4 {
    type Output = Vector4;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::Output {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
            w: self.w * rhs,
        }
    }
}
impl std::ops::Mul<Vector4> for f32 {
    type Output = Vector4;

    fn mul(self, rhs: Vector4) -> Self::Output {
        Self::Output {
            x: self * rhs.x,
            y: self * rhs.y,
            z: self * rhs.z,
            w: self * rhs.w,
        }
    }
}
impl std::ops::DivAssign<f32> for Vector4 {
    fn div_assign(&mut self, rhs: f32) {
        self.x = self.x / rhs;
        self.y = self.y / rhs;
        self.z = self.z / rhs;
        self.w = self.w / rhs;
    }
}
