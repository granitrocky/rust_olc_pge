use super::{pixel::Pixel, renderer::Renderer, util::ImageLoader};
#[derive(Clone, Default)]
pub struct Sprite {
    pub mode_sample: SpriteMode,
    pub width: u32,
    pub height: u32,
    pub col_data: Vec<Pixel>,
}

impl Sprite {
    pub fn new(width: u32, height: u32) -> Sprite {
        let image_size = (width * height) as usize;
        Sprite {
            mode_sample: SpriteMode::Normal,
            width,
            height,
            col_data: vec![Pixel::BLANK; image_size],
        }
    }

    pub fn new_with_data(width: u32, height: u32, col_data: Vec<Pixel>) -> Sprite {
        let image_size = (width * height) as usize;
        Sprite {
            mode_sample: SpriteMode::Normal,
            width,
            height,
            col_data,
        }
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> Pixel {
        match self.mode_sample {
            SpriteMode::Normal => {
                if x < self.width && y < self.height {
                    let index: usize = (y * self.width + x) as usize;
                    self.col_data[index]
                } else {
                    Pixel::rgb(0, 0, 0)
                }
            }
            SpriteMode::Periodic => {
                let index: usize = ((y % self.height) * self.width + (x % self.width)) as usize;
                self.col_data[index]
            }
        }
    }

    pub fn get_region(&self, pos: super::util::Vi2d, region: super::util::Vi2d) -> Vec<Pixel> {
        (pos.y..region.y + pos.y).fold(vec![], |mut v, r| {
            v.extend_from_slice(
                &self.col_data[(r as u32 * self.width + pos.x as u32) as usize
                    ..(r as u32 * self.width + (pos.x + region.x) as u32) as usize],
            );
            v
        })
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, p: Pixel) -> bool {
        if x < self.width && y < self.height {
            self.col_data[(y * self.width + x) as usize] = p;
            true
        } else {
            false
        }
    }
    pub fn set_region(&mut self, x: u32, y: u32, width: u32, height: u32, p: &[Pixel]) -> bool {
        todo!();
        if (x + width) < self.width && (y + height) < self.height {
            //Renderer::update_texture_region(0, x, y, width, height, p);
            true
        } else {
            false
        }
    }

    pub fn sample(&self, x: f32, y: f32) -> Pixel {
        let sx: u32 = std::cmp::min((x * self.width as f32) as u32, self.width - 1);
        let sy: u32 = std::cmp::min((y * self.height as f32) as u32, self.height - 1);
        self.get_pixel(sx, sy)
    }

    pub fn clear(&mut self, p: Pixel) {
        self.col_data.iter_mut().for_each(|c| *c = p);
    }

    pub fn sample_bl(u: f32, v: f32) -> Pixel {
        Pixel::rgb(0, 0, 0)
    }

    pub fn get_data(&self) -> &[u8] {
        let p_ptr = self.col_data.as_slice() as *const _ as *const u8;
        unsafe { std::slice::from_raw_parts(p_ptr, self.col_data.len() * 4) }
    }

    pub fn set_data(&mut self, data: &[u8], stride: u32) {
        self.col_data = data
            .chunks(stride as usize)
            .into_iter()
            .map(|c| {
                Pixel::raw(
                    c.iter()
                        .take(stride.min(4) as usize)
                        .enumerate()
                        .fold(0xFF000000, |out, (i, u)| out | (*u as u32) << (i as u32 * 8)),
                )
            })
            .collect();
    }

    /*pub fn as_texture(&self) -> u32{
        let tex_id = Renderer::create_texture(self.width, self.height);
        Renderer::apply_texture(tex_id);
        Renderer::update_texture(0, self);
        tex_id
    }*/

    pub fn overwrite_from_file<T: ImageLoader>(&mut self, file_path: &str) {
        if let Ok(spr) = T::load_image_resource(file_path) {
            *self = spr;
        }
    }

    pub fn load_from_file<T: ImageLoader>(file_path: &str) -> Option<Self> {
        if let Ok(spr) = T::load_image_resource(file_path) {
            Some(spr)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpriteMode {
    Normal,
    Periodic,
}
impl Default for SpriteMode {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpriteFlip {
    None,
    Horiz,
    Vert,
}
