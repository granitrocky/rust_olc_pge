pub const DEFAULT_ALPHA: u8 = 0xFF;
pub const DEFAULT_PIXEL: u32 = 0xFF << 24;

#[repr(C)]
#[derive(Clone, Copy)]
pub union Pixel {
    n: u32,
    rgba: (u8, u8, u8, u8),
}

unsafe impl bytemuck::Pod for Pixel {}
unsafe impl bytemuck::Zeroable for Pixel {}

impl Default for Pixel {
    fn default() -> Self {
        Self { n: DEFAULT_PIXEL }
    }
}

impl std::fmt::Debug for Pixel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { write!(f, "{:#X}", self.n) }
    }
}

impl PartialEq for Pixel {
    fn eq(&self, other: &Self) -> bool {
        self.r() == other.r()
            && self.g() == other.g()
            && self.b() == other.b()
            && self.a() == other.a()
    }
}

impl Pixel {
    pub const GREY: Pixel = Pixel::rgb(192, 192, 192);
    pub const DARK_GREY: Pixel = Pixel::rgb(128, 128, 128);
    pub const VERY_DARK_GREY: Pixel = Pixel::rgb(64, 64, 64);
    pub const RED: Pixel = Pixel::rgb(255, 0, 0);
    pub const DARK_RED: Pixel = Pixel::rgb(128, 0, 0);
    pub const VERY_DARK_RED: Pixel = Pixel::rgb(64, 0, 0);
    pub const YELLOW: Pixel = Pixel::rgb(255, 255, 0);
    pub const DARK_YELLOW: Pixel = Pixel::rgb(128, 128, 0);
    pub const VERY_DARK_YELLOW: Pixel = Pixel::rgb(64, 64, 0);
    pub const GREEN: Pixel = Pixel::rgb(0, 255, 0);
    pub const DARK_GREEN: Pixel = Pixel::rgb(0, 128, 0);
    pub const VERY_DARK_GREEN: Pixel = Pixel::rgb(0, 64, 0);
    pub const CYAN: Pixel = Pixel::rgb(0, 255, 255);
    pub const DARK_CYAN: Pixel = Pixel::rgb(0, 128, 128);
    pub const VERY_DARK_CYAN: Pixel = Pixel::rgb(0, 64, 64);
    pub const BLUE: Pixel = Pixel::rgb(0, 0, 255);
    pub const DARK_BLUE: Pixel = Pixel::rgb(0, 0, 128);
    pub const VERY_DARK_BLUE: Pixel = Pixel::rgb(0, 0, 64);
    pub const MAGENTA: Pixel = Pixel::rgb(255, 0, 255);
    pub const DARK_MAGENTA: Pixel = Pixel::rgb(128, 0, 128);
    pub const VERY_DARK_MAGENTA: Pixel = Pixel::rgb(64, 0, 64);
    pub const WHITE: Pixel = Pixel::rgb(255, 255, 255);
    pub const BLACK: Pixel = Pixel::rgb(0, 0, 0);
    pub const BLANK: Pixel = Pixel::rgba(0, 0, 0, 0);
}

impl From<(u8, u8, u8)> for Pixel {
    fn from(t: (u8, u8, u8)) -> Self {
        Pixel::rgb(t.0, t.1, t.2)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PixelMode {
    Normal,
    Mask,
    Alpha,
    Custom,
}

impl Pixel {
    /// Creates a new pixel with RGBA value.
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            n: (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | ((a as u32) << 24),
        }
    }

    /*    pub fn rand(rng: &mut ThreadRng) -> Self {
        let r: u32 = rng.gen_range(0, 255);
        let g: u32 = rng.gen_range(0, 255);
        let b: u32 = rng.gen_range(0, 255);
        let a: u32 = DEFAULT_ALPHA as u32;
        Self { n: (r | (g << 8) | (b << 16) | (a << 24)) as u32 }
    }*/

    /// Creates a new pixel with RGB value.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            n: (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | ((DEFAULT_ALPHA as u32) << 24),
        }
    }
    /// Creates a new pixel with RGBA value.
    pub const fn raw(n: u32) -> Self {
        Self { n }
    }

    pub fn blend_add(colors: &[Pixel]) -> Pixel {
        let (mut r, mut g, mut b, mut a): (u8, u8, u8, u8) = (0x00, 0x00, 0x00, 0x00);
        for p in colors.iter() {
            unsafe {
                r = std::cmp::min(r + p.rgba.0, 255);
                g = std::cmp::min(g + p.rgba.1, 255);
                b = std::cmp::min(b + p.rgba.2, 255);
                a = std::cmp::min(a + p.rgba.3, 255);
            }
        }
        Pixel::rgba(r, g, b, a)
    }
    pub fn blend_div(colors: &[Pixel]) -> Pixel {
        let (mut r, mut g, mut b, mut a, mut index): (u8, u8, u8, u8, u8) =
            (0x00, 0x00, 0x00, 0x00, 0);
        for p in colors.iter() {
            if p != &Pixel::BLANK {
                unsafe {
                    r += p.rgba.0;
                    g += p.rgba.1;
                    b += p.rgba.2;
                    a += p.rgba.3;
                }
                index += 1;
            }
        }
        if index > 0 {
            Pixel::rgba(r / index as u8, g / index as u8, b / index as u8, a)
        } else {
            Pixel::BLANK
        }
    }
    pub fn blend_mul(&self, rhs: &Pixel) -> Pixel {
        unsafe {
            let mut n = self.n >> 1;
            let mut n2 = rhs.n >> 1;
            //Alpha components / 2
            let a1 = n & (0xFF000000 >> 1);
            let a2 = n2 & (0xFF000000 >> 1);
            //Color components / 2
            n &= !0xFF000000 >> 1;
            n2 &= !0xFF000000 >> 1;
            //
            n = (n + n2) ^ ((a1 + a2) & 0xFF000000);
            Pixel::raw(n)
        }
    }

    pub fn alpha_blend(&self, rhs: &Pixel) -> Pixel {
        unsafe {
            let a1 = self.a() as f32 / 255.0;
            let a2 = rhs.a() as f32 / 255.0;
            if a1 == 0.0 {
                return *rhs;
            }

            let a1p = 1.0 - a1;
            let (r1, g1, b1) = (
                self.r() as f32 * a1,
                self.g() as f32 * a1,
                self.b() as f32 * a1,
            );
            let (r2, g2, b2) = (
                (rhs.r() as f32 * a2) * a1p,
                (rhs.g() as f32 * a2) * a1p,
                (rhs.b() as f32 * a2) * a1p,
            );
            Pixel::rgb(
                (r1 + r2) as u8,
                (g1 + g2) as u8,
                (b1 + b2) as u8,
                //(a1 + a2) as u8,
            )
        }
    }

    pub fn r(&self) -> u8 {
        unsafe { self.rgba.0 }
    }
    pub fn g(&self) -> u8 {
        unsafe { self.rgba.1 }
    }
    pub fn b(&self) -> u8 {
        unsafe { self.rgba.2 }
    }
    pub fn a(&self) -> u8 {
        unsafe { self.rgba.3 }
    }
}

impl std::ops::Mul<f32> for Pixel {
    type Output = Pixel;

    fn mul(self, rhs: f32) -> Self::Output {
        unsafe {
            let (r, g, b) = (
                (self.r() as f32 * rhs).min(255.0) as u8,
                (self.g() as f32 * rhs).min(255.0) as u8,
                (self.b() as f32 * rhs).min(255.0) as u8,
            );
            Pixel {
                rgba: (r, g, b, self.a()),
            }
        }
    }
}
impl std::ops::Mul for Pixel {
    type Output = Pixel;

    fn mul(self, rhs: Pixel) -> Self::Output {
        self.alpha_blend(&rhs)
    }
}
