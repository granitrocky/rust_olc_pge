use super::{
    olc::Rcode,
    pixel::Pixel,
    sprite::Sprite,
};
use std::{ops, fmt};
#[cfg(target_arch = "wasm32")]
use web_sys::{Request, Response};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;

impl<T> V2d<T> {
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl Vi2d {
    /// Returns magnitude (or length) of a vector.
    pub fn mag(&self) -> i32 {
        (self.mag2() as f32).sqrt() as i32
    }

    /// Returns magnitude squared.
    pub fn mag2(&self) -> i32 {
        self.x * self.x + self.y * self.y
    }

    /// Returns vector norm.
    pub fn norm(&self) -> Self {
        let r = 1 / self.mag();
        Self {
            x: self.x * r,
            y: self.y * r,
        }
    }

    /// Returns perpendicular vector.
    pub fn perp(&self) -> Self {
        Self {
            x: -self.y,
            y: self.x,
        }
    }

    /// Returns dot product of two vectors.
    pub fn dot(&self, rhs: Vi2d) -> i32 {
        self.x * rhs.x + self.y * rhs.y
    }

    /// Returns cross product of two vectors.
    pub fn cross(&self, rhs: Vi2d) -> i32 {
        self.x * rhs.y - self.y * rhs.x
    }

    pub fn to_vf2d(self) -> Vf2d {
        Vf2d::new(self.x as f32, self.y as f32)
    }
}

impl Vf2d {
    /// Returns magnitude (or length) of a vector.
    pub fn mag(&self) -> f32 {
        self.mag2().sqrt()
    }

    /// Returns magnitude squared.
    pub fn mag2(&self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// Returns vector norm.
    pub fn norm(&self) -> Self {
        let r = 1.0 / self.mag();
        Self {
            x: self.x * r,
            y: self.y * r,
        }
    }

    /// Returns perpendicular vector.
    pub fn perp(&self) -> Self {
        Self {
            x: -self.y,
            y: self.x,
        }
    }

    /// Returns dot product of two vectors.
    pub fn dot(&self, rhs: Vf2d) -> f32 {
        self.x * rhs.x + self.y * rhs.y
    }

    /// Returns cross product of two vectors.
    pub fn cross(&self, rhs: Vf2d) -> f32 {
        self.x * rhs.y - self.y * rhs.x
    }

    pub fn to_vi2d(self) -> Vi2d {
        Vi2d::new(self.x as i32, self.y as i32)
    }
}

impl<T> From<(T, T)> for V2d<T> {
    fn from(tuple: (T, T)) -> Self {
        Self {
            x: tuple.0,
            y: tuple.1,
        }
    }
}

impl<T: ops::Add<Output = T>> ops::Add for V2d<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}
impl<T: ops::Add<Output = T> + Copy> ops::Add<T> for V2d<T> {
    type Output = Self;
    fn add(self, other: T) -> Self::Output {
        Self {
            x: self.x + other,
            y: self.y + other,
        }
    }
}

impl<T: ops::AddAssign> ops::AddAssign for V2d<T> {
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
    }
}
impl<T: ops::AddAssign + Copy> ops::AddAssign<T> for V2d<T> {
    fn add_assign(&mut self, other: T) {
        self.x += other;
        self.y += other;
    }
}

impl<T: ops::Sub<Output = T>> ops::Sub for V2d<T> {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}
impl<T: ops::Sub<Output = T> + Copy> ops::Sub<T> for V2d<T> {
    type Output = Self;

    fn sub(self, other: T) -> Self::Output {
        Self {
            x: self.x - other,
            y: self.y - other,
        }
    }
}

impl<T: ops::SubAssign> ops::SubAssign for V2d<T> {
    fn sub_assign(&mut self, other: Self) {
        self.x -= other.x;
        self.y -= other.y;
    }
}
impl<T: ops::SubAssign + Copy> ops::SubAssign<T> for V2d<T> {
    fn sub_assign(&mut self, other: T) {
        self.x -= other;
        self.y -= other;
    }
}

impl<T: ops::Mul<Output = T>> ops::Mul for V2d<T> {
    type Output = Self;

    fn mul(self, other: Self) -> Self::Output {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
        }
    }
}
impl<T: ops::Mul<Output = T> + Copy> ops::Mul<T> for V2d<T> {
    type Output = Self;

    fn mul(self, other: T) -> Self::Output {
        Self {
            x: self.x * other,
            y: self.y * other,
        }
    }
}

impl<T: ops::MulAssign> ops::MulAssign for V2d<T> {
    fn mul_assign(&mut self, other: Self) {
        self.x *= other.x;
        self.y *= other.y;
    }
}

impl<T: ops::MulAssign + Copy> ops::MulAssign<T> for V2d<T> {
    fn mul_assign(&mut self, other: T) {
        self.x *= other;
        self.y *= other;
    }
}

impl<T: ops::Div<Output = T>> ops::Div for V2d<T> {
    type Output = Self;

    fn div(self, other: Self) -> Self::Output {
        Self {
            x: self.x / other.x,
            y: self.y / other.y,
        }
    }
}
impl<T: ops::Div<Output = T> + Copy> ops::Div<T> for V2d<T> {
    type Output = Self;

    fn div(self, other: T) -> Self::Output {
        Self {
            x: self.x / other,
            y: self.y / other,
        }
    }
}

impl<T: ops::DivAssign> ops::DivAssign for V2d<T> {
    fn div_assign(&mut self, other: Self) {
        self.x /= other.x;
        self.y /= other.y;
    }
}
impl<T: ops::DivAssign + Copy> ops::DivAssign<T> for V2d<T> {
    fn div_assign(&mut self, other: T) {
        self.x /= other;
        self.y /= other;
    }
}

impl<T: fmt::Display + fmt::Debug> fmt::Display for V2d<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:?}, {:?})", self.x, self.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HWButton {
    /// Set once during the frame the event occurs.
    pub pressed: bool,
    /// Set once during the frame the event occurs.
    pub released: bool,
    /// Set true for all frames between pressed and released events.
    pub held: bool,
}

impl Default for HWButton {
    fn default() -> Self {
        Self::new()
    }
}

impl HWButton {
    pub fn new() -> Self {
        HWButton {
            pressed: false,
            released: false,
            held: false,
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mouse {
    Left,
    Right,
    Middle,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V2d<T> {
    pub x: T,
    pub y: T,
}

pub type Vi2d = V2d<i32>;
pub type Vf2d = V2d<f32>;


pub trait ImageLoader {
    fn load_image_resource(image_file: &str) -> Result<Sprite, Rcode>;
    fn load_image_from_bytes(bytes: &[u8]) -> Result<Sprite, Rcode>;
    fn save_image_resource(spr: Sprite, image_file: &str) -> Result<(),Rcode>;
}

pub struct BMPLoader;
pub struct PNGLoader;

pub struct ResourceBuffer {}

pub struct ResourcePack {}

impl ImageLoader for BMPLoader {
    fn load_image_resource(image_file: &str) -> Result<Sprite,Rcode> {
        let image_path = std::path::Path::new(image_file);
        if !image_path.exists() {
            return Err(Rcode::NoFile);
        };
        let img = bmp::open(image_path).unwrap_or_else(|e| bmp::Image::new(0, 0));
        if img.get_width() == 0 || img.get_height() == 0 {
            return Err(Rcode::Fail);
        }
        let mut spr = Sprite::new(
        img.get_width(),
        img.get_height());
        //No Alpha for now because BMP is a dumb format
        spr.col_data = vec![Pixel::rgb(0, 0, 0); (spr.width * spr.height) as usize];
        for y in 0..spr.height {
            for x in 0..spr.width {
                let p = img.get_pixel(x, y);
                spr.set_pixel(x, y, Pixel::rgb(p.r, p.g, p.b));
            }
        }
        Ok(spr)
    }

    fn load_image_from_bytes(bytes: &[u8]) -> Result<Sprite, Rcode>{
        Err(Rcode::Fail)
    }

    fn save_image_resource(spr: Sprite, image_file: &str) -> Result<(), Rcode> {
        Ok(())
    }
}


impl ImageLoader for PNGLoader {
    fn load_image_resource(image_file: &str) -> Result<Sprite, Rcode> {
        use image::io::Reader as ImageReader;
        let image_path = std::path::Path::new(image_file);
        if !image_path.exists() {
            return Err(Rcode::NoFile);
        }

        let mut img = if let Ok(reader) = ImageReader::open(image_path){
            reader.decode().expect("Can't decode the image").into_rgba()
        } else{
            return Err(Rcode::Fail);
        };
        let mut spr = Sprite::new(
        img.width(),
        img.height());
        spr.col_data = vec![Pixel::BLANK; (spr.width * spr.height) as usize];
        for y in 0..spr.height as usize {
            for x in 0..spr.width as usize {
                let c = img.get_pixel(x as u32, y as u32);
                spr.set_pixel(
                    x as u32,
                    y as u32,
                    Pixel::rgba(c[0], c[1], c[2], c[3]),
                );
            }
        }
        Ok(spr)
    }

    fn load_image_from_bytes(bytes: &[u8]) -> Result<Sprite, Rcode>{
        use image::io::Reader as ImageReader;

        let mut reader = ImageReader::new(std::io::Cursor::new(bytes));
        reader.set_format(image::ImageFormat::Png);
        let image = reader.decode();
        let mut img = image.expect("NAAAAAA").into_rgba();
        let mut spr = Sprite::new(
        img.width(),
        img.height());
        spr.col_data = vec![Pixel::BLANK; (spr.width * spr.height) as usize];
        for y in 0..spr.height as usize {
            for x in 0..spr.width as usize {
                let c = img.get_pixel(x as u32, y as u32);
                spr.set_pixel(
                    x as u32,
                    y as u32,
                    Pixel::rgba(c[0], c[1], c[2], c[3]),
                );
            }
        }
        Ok(spr)
    }

    fn save_image_resource(spr: Sprite, image_file: &str) -> Result<(), Rcode> {
        Ok(())
    }
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

#[cfg(target_arch = "wasm32")]
pub async fn get_file_as_u8(path: &str) -> Vec<u8> {
    use wasm_bindgen::JsCast;
    let window = web_sys::window().unwrap();
    let request = Request::new_with_str(path).unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .expect("No Future");
    let resp: Response = resp_value.dyn_into().unwrap();
    let buffer: js_sys::ArrayBuffer = JsFuture::from(resp.array_buffer().unwrap())
        .await
        .unwrap()
        .dyn_into()
        .unwrap();
    js_sys::Uint8Array::new_with_byte_offset_and_length(&buffer, 0, buffer.byte_length() as u32)
        .to_vec()
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_file_as_u8(path: &str) -> Vec<u8> {
    std::fs::read(path).expect("File does not exist")
}

