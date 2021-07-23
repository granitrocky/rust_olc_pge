#![allow(non_snake_case)]
use std::cmp::Ordering;

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct BiVector3 {
    pub b12: f32,
    pub b13: f32,
    pub b23: f32,
}

#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub struct Rotor3 {
    pub a: f32,
    pub b12: f32,
    pub b13: f32,
    pub b23: f32,
}

impl Default for Rotor3 {
    fn default() -> Self {
        (1.0, 0.0, 0.0, 0.0).into()
    }
}

impl From<(f32, f32, f32)> for BiVector3 {
    fn from(tuple: (f32, f32, f32)) -> Self {
        BiVector3 {
            b12: tuple.0,
            b13: tuple.1,
            b23: tuple.2,
        }
    }
}
impl From<(Vector3, Vector3)> for BiVector3 {
    fn from(tuple: (Vector3, Vector3)) -> Self {
        outer(tuple.0, tuple.1)
    }
}
impl From<(f32, f32, f32)> for Vector3 {
    fn from(tuple: (f32, f32, f32)) -> Self {
        Vector3 {
            x: tuple.0,
            y: tuple.1,
            z: tuple.2,
        }
    }
}

impl From<&[f32; 3]> for Vector3 {
    fn from(tuple: &[f32; 3]) -> Self {
        Vector3 {
            x: tuple[0],
            y: tuple[1],
            z: tuple[2],
        }
    }
}

impl Into<[f32; 3]> for Vector3 {
    fn into(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
}

impl From<[f32; 3]> for Vector3 {
    fn from(tuple: [f32; 3]) -> Self {
        Vector3 {
            x: tuple[0],
            y: tuple[1],
            z: tuple[2],
        }
    }
}

impl std::cmp::PartialEq for Rotor3 {
    fn eq(&self, other: &Self) -> bool {
        (self.a - other.a).abs() < f32::EPSILON
            && (self.b12 - other.b12).abs() < f32::EPSILON
            && (self.b13 - other.b13).abs() < f32::EPSILON
            && (self.b23 - other.b23).abs() < f32::EPSILON
    }
}

impl From<(f32, f32, f32, f32)> for Rotor3 {
    fn from(tuple: (f32, f32, f32, f32)) -> Self {
        Rotor3 {
            a: tuple.0,
            b12: tuple.1,
            b13: tuple.2,
            b23: tuple.3,
        }
    }
}

impl From<[f32; 4]> for Rotor3 {
    fn from(arr: [f32; 4]) -> Self {
        Self {
            a: arr[0],
            b12: arr[1],
            b13: arr[2],
            b23: arr[3],
        }
    }
}

pub fn outer(u: Vector3, v: Vector3) -> BiVector3 {
    (
        u.x * v.y - u.y * v.x,
        u.x * v.z - u.z * v.x,
        u.y * v.z - u.z * v.y,
    )
        .into()
}

impl Rotor3 {
    pub fn turn_left(&self) -> Rotor3 {
        *self * Rotor3::from_vectors(*self * (0.0, 0.0, 1.0), *self * (-1.0, 0.0, 0.0))
    }
    pub fn turn_right(&self) -> Rotor3 {
        *self * Rotor3::from_vectors(*self * (0.0, 0.0, 1.0), *self * (1.0, 0.0, 0.0))
    }
    pub fn turn_up(&self) -> Rotor3 {
        *self * Rotor3::from_vectors(*self * (0.0, 0.0, 1.0), *self * (0.0, 1.0, 0.0))
    }
    pub fn turn_down(&self) -> Rotor3 {
        *self * Rotor3::from_vectors(*self * (0.0, 0.0, 1.0), *self * (0.0, -1.0, 0.0))
    }

    pub fn left(&self) -> Vector3 {
        *self * (-1.0, 0.0, 0.0)
    }
    pub fn right(&self) -> Vector3 {
        *self * (1.0, 0.0, 0.0)
    }
    pub fn up(&self) -> Vector3 {
        *self * (0.0, 1.0, 0.0)
    }
    pub fn down(&self) -> Vector3 {
        *self * (0.0, -1.0, 0.0)
    }
    pub fn forward(&self) -> Vector3 {
        *self * (0.0, 0.0, 1.0)
    }
    pub fn backward(&self) -> Vector3 {
        *self * (0.0, 0.0, -1.0)
    }

    pub fn from_bivector(mut bv: BiVector3) -> Self {
        let a = 1.0 - bv.length();
        Self {
            a: 0.0_f32.max(a),
            b12: bv.b12,
            b13: bv.b13,
            b23: bv.b23,
        }.normal()
    }

    pub fn from_vectors(vFrom: Vector3, vTo: Vector3) -> Self {
        //This 1 + dot product will give you half the angle
        let k_cos_theta = vTo.dot(vFrom);
        let k = (vFrom.length_sqrd() * vTo.length_sqrd()).sqrt();

        if ((k_cos_theta / k) + 1.0).abs() < f32::EPSILON {
            //Run check for orthogonal rotations
        }

        let minus_b = outer(vTo, vFrom);
        Rotor3 {
            a: k_cos_theta + k,
            b12: minus_b.b12,
            b13: minus_b.b13,
            b23: minus_b.b23,
        }
        .normal()
    }

    pub fn from_angle_and_axis(angleRadian: f32, mut bvPlane: BiVector3) -> Self {
        let angle_half = angleRadian / 2.0;
        bvPlane.normalize();
        let sina = angle_half.sin();
        Rotor3 {
            a: angle_half.cos(),
            b12: -sina * bvPlane.b12,
            b13: -sina * bvPlane.b13,
            b23: -sina * bvPlane.b23,
        }
        .normal()
    }

    pub fn rotate_vector(&self, v: Vector3) -> Vector3 {
        let q = Vector3 {
            x: self.a * v.x - v.y * self.b12 - v.z * self.b13,
            y: self.a * v.y + v.x * self.b12 - v.z * self.b23,
            z: self.a * v.z + v.x * self.b13 + v.y * self.b23,
        };

        let q321: f32 = v.x * self.b23 - v.y * self.b13 + v.z * self.b12;

        Vector3 {
            x: self.a * q.x - q.y * self.b12 - q.z * self.b13 + q321 * self.b23,
            y: self.a * q.y + q.x * self.b12 - q.z * self.b23 - q321 * self.b13,
            z: self.a * q.z + q.x * self.b13 + q.y * self.b23 + q321 * self.b12,
        }
    }
    pub fn rotate_by_rotor(&mut self, r: Rotor3){
       *self = (*self * r * (*self).reverse()).normal();

    }
    pub fn reverse(&self) -> Rotor3 {
        Rotor3 {
            a: self.a,
            b12: -self.b12,
            b13: -self.b13,
            b23: -self.b23,
        }
    }

    pub fn length_sqrd(&self) -> f32 {
        self.a * self.a + self.b12 * self.b12 + self.b13 * self.b13 + self.b23 * self.b23
    }

    pub fn length(&self) -> f32 {
        self.length_sqrd().sqrt()
    }

    pub fn mag(&self) -> f32 {
        self.b12 * self.b12 + self.b13 * self.b13 + self.b23 * self.b23
    }

    pub fn normalize(&mut self) {
        let n = self.normal();
        self.a = n.a;
        self.b12 = n.b12;
        self.b13 = n.b13;
        self.b23 = n.b23;
    }
    pub fn normal(&self) -> Rotor3 {
        let l = self.length();
        Rotor3 {
            a: self.a / l,
            b12: self.b12 / l,
            b13: self.b13 / l,
            b23: self.b23 / l,
        }
    }

    pub fn get_axle(&self, v: Vector3) -> Rotor3 {
        let angle = (self.a * 2.0).acos();
        Rotor3::default()
    }

    pub fn slerp(&self, v2: Rotor3, s: f32) -> Rotor3 {
        let from_rev = self.reverse();
        ((v2 * from_rev) * s.clamp(0.0, 1.0)) * *self
    }

    pub fn to_matrix3(self) -> cgmath::Matrix3<f32> {
        let v0 = Vector3::new(1.0, 0.0, 0.0) * self;
        let v1 = Vector3::new(0.0, 1.0, 0.0) * self;
        let v2 = Vector3::new(0.0, 0.0, 1.0) * self;

        cgmath::Matrix3::new(v0.x, v0.y, v0.z, v1.x, v1.y, v1.z, -v2.x, -v2.y, -v2.z)
    }

    pub fn from_quat(q: [f32; 4]) -> Rotor3 {
        Rotor3 {
            a: q[3],
            b12: q[2],
            b13: q[1],
            b23: q[0],
        }
    }

    /*
    // geometric product (for reference), produces twice the angle, negative direction
    inline Rotor3 Geo( const Vector3 & a, const Vector3 & b )
    {
        return Rotor3( Dot(a,b), Wedge(a,b) );
    }
    */
}

impl std::ops::Mul for Rotor3 {
    type Output = Rotor3;

    fn mul(self, rhs: Self) -> Self::Output {
        Rotor3 {
            a: self.a * rhs.a -     //1a  2a
                self.b12 * rhs.b12 -//1xy 2xy
                self.b13 * rhs.b13 -//1xz 2xz
                self.b23 * rhs.b23, //1yz 2yz

            b12: self.b12 * rhs.a + //1xy 2a
                self.a * rhs.b12 +  //1a  2xy
                self.b23 * rhs.b13 -//1yz 2xz
                self.b13 * rhs.b23, //1xz 2yz

            b13: self.b13 * rhs.a + //1xz 2a
                self.a * rhs.b13 +  //1a  2xz
                self.b12 * rhs.b23 - //1xy 2yz
                self.b23 * rhs.b12, //1yz 2xy

            b23: self.b23 * rhs.a + //1yz 2a
                self.a * rhs.b23 +  //1a  2yz
                self.b13 * rhs.b12 -//1xz 2xy
                self.b12 * rhs.b13, //1xy 2xz
        }
    }
}
impl std::ops::Mul<f32> for Rotor3 {
    type Output = Rotor3;

    fn mul(self, rhs: f32) -> Self::Output {
        let bv: BiVector3 = (self.b12 * rhs, self.b13 * rhs, self.b23 * rhs).into();
        Rotor3::from_bivector(bv)
    }
}

impl std::ops::MulAssign<f32> for Rotor3 {
    fn mul_assign(&mut self, rhs: f32) {
        let bv: BiVector3 = (self.b12 * rhs, self.b13 * rhs, self.b23 * rhs).into();
        *self = Rotor3::from_bivector(bv)
    }
}
impl std::ops::MulAssign for Rotor3 {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}
impl std::ops::Mul<Vector3> for Rotor3 {
    type Output = Vector3;

    fn mul(self, rhs: Vector3) -> Self::Output {
        self.rotate_vector(rhs)
    }
}
impl std::ops::Mul<(f32, f32, f32)> for Rotor3 {
    type Output = Vector3;

    fn mul(self, rhs: (f32, f32, f32)) -> Self::Output {
        self.rotate_vector(rhs.into())
    }
}

impl BiVector3 {
    pub fn normalize(&mut self) {
        let l = self.length();
        self.b12 /= l;
        self.b13 /= l;
        self.b23 /= l;
    }
    pub fn length(&self) -> f32 {
        self.length_sqrd().sqrt()
    }
    pub fn length_sqrd(&self) -> f32 {
        self.b12 * self.b12 + self.b13 * self.b13 + self.b23 * self.b23
    }
    pub fn as_vector3(&self) -> Vector3 {
        Vector3::new(self.b23, self.b13, self.b23)
    }

    pub fn from_basis(bx: Vector3, by: Vector3, bz: Vector3) -> BiVector3 {
        BiVector3::default()
    }
}

impl Vector3 {
    pub fn left() -> Vector3 {
        (-1.0, 0.0, 0.0).into()
    }
    pub fn right() -> Vector3 {
        (1.0, 0.0, 0.0).into()
    }
    pub fn up() -> Vector3 {
        (0.0, 1.0, 0.0).into()
    }
    pub fn down() -> Vector3 {
        (0.0, -1.0, 0.0).into()
    }
    pub fn forward() -> Vector3 {
        (0.0, 0.0, 1.0).into()
    }
    pub fn backward() -> Vector3 {
        (0.0, 0.0, -1.0).into()
    }

    pub fn one() -> Vector3 {
        (1.0, 1.0, 1.0).into()
    }

    pub fn max() -> Vector3 {
        (f32::MAX, f32::MAX, f32::MAX).into()
    }

    pub fn new(x: f32, y: f32, z: f32) -> Vector3 {
        Vector3 { x, y, z }
    }
    pub fn dot(&self, rhs: Vector3) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }
    pub fn cross(&self, rhs: Vector3) -> Self {
        Vector3 {
            x: self.y * rhs.z - self.z * rhs.y,
            y: self.z * rhs.x - self.x * rhs.z,
            z: self.x * rhs.y - self.y * rhs.x,
        }
    }
    pub fn normal(&self) -> Vector3 {
        let l = (self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        *self / l
    }
    pub fn length(&self) -> f32 {
        self.length_sqrd().sqrt()
    }
    pub fn length_sqrd(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn is_valid(&self) -> bool {
        !(self.x.is_nan() || self.y.is_nan() || self.z.is_nan())
    }
}

impl Eq for Vector3{}

impl Ord for Vector3{
    fn cmp(&self, v2: &Self) -> std::cmp::Ordering {
        let a = self.length_sqrd();
        let b = self.length_sqrd();
        if a.round_to(5) < b.round_to(5) {
            std::cmp::Ordering::Less
        } else if a.round_to(5) > b.round_to(5) {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    }
}

pub struct Mat4x4 {
    pub m: [[f32; 4]; 4],
}

impl Mat4x4 {
    pub fn multiply_vector(&self, v: Vector3) -> Vector3 {
        let w = v.x * self.m[0][3] + v.y * self.m[1][3] + v.z * self.m[2][3] + self.m[3][3];
        let mut r_v: Vector3 = (
            v.x * self.m[0][0] + v.y * self.m[1][0] + v.z * self.m[2][0] + self.m[3][0],
            v.x * self.m[0][1] + v.y * self.m[1][1] + v.z * self.m[2][1] + self.m[3][1],
            v.x * self.m[0][2] + v.y * self.m[1][2] + v.z * self.m[2][2] + self.m[3][2],
        )
            .into();
        if w != 0.0 {
            r_v /= w;
        }
        r_v
    }
}

impl From<[[f32; 4]; 4]> for Mat4x4 {
    fn from(m: [[f32; 4]; 4]) -> Self {
        Self { m }
    }
}

impl std::ops::Add for Vector3 {
    type Output = Vector3;

    fn add(self, rhs: Self) -> Self::Output {
        Self::Output {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl std::ops::Add<f32> for Vector3 {
    type Output = Vector3;

    fn add(self, rhs: f32) -> Self::Output {
        Self::Output {
            x: self.x + rhs,
            y: self.y + rhs,
            z: self.z + rhs,
        }
    }
}

impl std::ops::Sub for Vector3 {
    type Output = Vector3;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::Output {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl std::ops::Sub<f32> for Vector3 {
    type Output = Vector3;

    fn sub(self, rhs: f32) -> Self::Output {
        Self::Output {
            x: self.x - rhs,
            y: self.y - rhs,
            z: self.z - rhs,
        }
    }
}

impl std::ops::Sub<[f32; 3]> for Vector3 {
    type Output = Vector3;

    fn sub(self, rhs: [f32; 3]) -> Self::Output {
        Self::Output {
            x: self.x - rhs[0],
            y: self.y - rhs[1],
            z: self.z - rhs[2],
        }
    }
}

impl std::ops::AddAssign for Rotor3 {
    fn add_assign(&mut self, rhs: Self) {
        self.b12 += rhs.b12;
        self.b13 += rhs.b13;
        self.b23 += rhs.b23;
        self.normalize();
    }
}

impl std::ops::Add for Rotor3 {
    type Output = Rotor3;

    fn add(self, rhs: Self) -> Self::Output {
        Self::Output {
            a: self.a,
            b12: self.b12 + rhs.b12,
            b13: self.b13 + rhs.b13,
            b23: self.b23 + rhs.b23,
        }
        .normal()
    }
}

impl std::ops::Sub for Rotor3 {
    type Output = Rotor3;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::Output {
            a: self.a - rhs.a,
            b12: self.b12 - rhs.b12,
            b13: self.b13 - rhs.b13,
            b23: self.b23 - rhs.b23,
        }
        .normal()
    }
}

impl std::ops::Div for Vector3 {
    type Output = Vector3;

    fn div(self, rhs: Self) -> Self::Output {
        let mut o_v = Self::Output {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
            z: self.z / rhs.z,
        };
        if o_v.x.is_nan() {
            o_v.x = 0.0;
        }
        if o_v.y.is_nan() {
            o_v.y = 0.0;
        }
        if o_v.z.is_nan() {
            o_v.z = 0.0;
        }
        o_v
    }
}

impl std::ops::Div<f32> for Vector3 {
    type Output = Vector3;

    fn div(self, rhs: f32) -> Self::Output {
        Self::Output {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}
impl std::ops::DivAssign<f32> for Vector3 {
    fn div_assign(&mut self, rhs: f32) {
        self.x = self.x / rhs;
        self.y = self.y / rhs;
        self.z = self.z / rhs;
    }
}
impl std::ops::Mul<f32> for Vector3 {
    type Output = Vector3;

    fn mul(self, rhs: f32) -> Self::Output {
        Vector3 {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl std::ops::Mul for Vector3 {
    type Output = Vector3;

    fn mul(self, rhs: Self) -> Self::Output {
        Vector3 {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
        }
    }
}

impl std::cmp::PartialEq for Vector3 {
    fn eq(&self, other: &Self) -> bool {
        (self.x - other.x).abs() < f32::EPSILON
            && (self.y - other.y).abs() < f32::EPSILON
            && (self.z - other.z).abs() < f32::EPSILON
    }
}
impl std::cmp::PartialOrd for Vector3 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let s = self.length_sqrd();
        let o = self.length_sqrd();
        if (s - o).abs() < f32::EPSILON {
            return Some(Ordering::Equal);
        }

        match s < o {
            true => Some(Ordering::Less),
            false => Some(Ordering::Greater),
        }
    }
}

impl std::ops::Mul<Vector3> for f32 {
    type Output = Vector3;

    fn mul(self, rhs: Vector3) -> Self::Output {
        Vector3 {
            x: rhs.x * self,
            y: rhs.y * self,
            z: rhs.z * self,
        }
    }
}
impl std::ops::MulAssign<f32> for Vector3 {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

impl std::ops::Mul<Rotor3> for Vector3 {
    type Output = Vector3;

    fn mul(self, rhs: Rotor3) -> Self::Output {
        rhs * self
    }
}

impl std::ops::AddAssign for Vector3 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl std::ops::SubAssign for Vector3 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

pub fn approx_equal(a: f32, b: f32, dp: u8) -> bool {
    let p: f32 = 10f32.powf(-(dp as f32));

    (a - b).abs() < p
}

pub trait RoundTo<T: num_traits::Float> {
    fn round_to(self, decimals: i32) -> T;
}

impl RoundTo<f64> for f64 {
    fn round_to(self, decimals: i32) -> f64 {
        (self * 10.0_f64.powi(decimals)).trunc() / 10.0_f64.powi(decimals)
    }
}

impl RoundTo<f32> for f32 {
    fn round_to(self, decimals: i32) -> f32 {
        (self * 10.0_f32.powi(decimals)).trunc() / 10.0_f32.powi(decimals)
    }
}

impl std::ops::Add for BiVector3 {
    type Output = BiVector3;

    fn add(self, rhs: BiVector3) -> Self::Output {
        (self.b12 + rhs.b12, self.b13 + rhs.b13, self.b23 + rhs.b23).into()
    }
}

// Taken from https://github.com/toji/gl-matrix/blob/master/src/quat.js
impl From<cgmath::Matrix3<f32>> for Rotor3 {
    fn from(m_a: cgmath::Matrix3<f32>) -> Self {
        // 0  1  2
        // 3  4  5
        // 6  7  8
        // 00 10 20
        // 01 11 21
        // 02 12 22
        let f_trace = m_a[0][0] + m_a[1][1] + m_a[2][2];
        let mut out = Rotor3::default();
        if f_trace > 0.0 {
            let mut f_root = (f_trace + 1.0).sqrt();
            //m[3]
            out.a = f_root * 0.5;
            f_root = 0.5 / f_root;
            //m[2]
            out.b12 = (m_a[1][0] - m_a[0][1]) * f_root;
            //m[1]
            out.b13 = (m_a[0][2] - m_a[2][0]) * f_root;
            //m[0]
            out.b23 = (m_a[2][1] - m_a[1][2]) * f_root;
        } else {
            let mut i = 0;
            let mut arr: [f32; 4] = [0.0; 4];
            if m_a[1][1] > m_a[0][0] {
                i = 1;
            }
            if m_a[2][2] > m_a[i][i] {
                i = 2;
            }
            let j = (i + 1) % 3;
            let k = (i + 2) % 3;

            let mut f_root = (m_a[i][i] - m_a[j][j] - m_a[k][k] + 1.0).sqrt();
            arr[i] = 0.5 * f_root;
            f_root = 0.5 / f_root;
            arr[3] = (m_a[j][k] - m_a[k][j]) * f_root;
            arr[j] = (m_a[j][i] + m_a[i][j]) * f_root;
            arr[k] = (m_a[k][i] + m_a[i][k]) * f_root;
            out.a = arr[3];
            out.b12 = arr[2];
            out.b13 = arr[1];
            out.b23 = arr[0];
        }
        out.normal()
    }
}
