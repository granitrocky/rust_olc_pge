use super::{
    sprite::Sprite,
    util::{Vf2d, Vi2d},
    renderer::Renderer,
    pixel::Pixel,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct SmallD {
    pub id: i32,
    pub sprite: Sprite,
    pub uv_scale: Vf2d,
}

#[derive(Clone)]
pub struct Decal {
    pub d_inst: Arc<SmallD>,
}

impl Drop for Decal {
    fn drop(&mut self) {
        let id = self.get().id;
        if id != -1 {
            Renderer::delete_texture(&mut (id as u32));
        }
    }
}

impl Decal {
    pub fn empty() -> Self {
        let small = SmallD {
            id: -1,
            sprite: Sprite::new(0, 0),
            uv_scale: Vf2d::from((1.0, 1.0)),
        };
        Self {
            d_inst: Arc::new(small),
        }
    }

    pub fn create(spr: Option<Sprite>, renderer: &mut Renderer) -> Self {
        match spr {
            Some(sprite) => {
                let mut small = SmallD {
                    id: renderer.create_texture(sprite.width, sprite.height),
                    sprite,
                    uv_scale: Vf2d::from((1.0, 1.0)),
                };
                //Decal::update(&mut small);
                Self {
                    d_inst: Arc::new(small),
                }
            }
            None => Decal::empty(),
        }
    }

    /*fn update(small: &mut SmallD) {
        if small.id == -1 { return; };
        small.uv_scale = Vf2d::from(
            (1.0 / (small.sprite.width as f32),
             (1.0 / (small.sprite.height as f32))
            ));
        Renderer::apply_texture(small.id as u32);
        Renderer::update_texture(small.id as u32, &small.sprite);
    }*/

    pub fn get(&self) -> Arc<SmallD> {
        Arc::clone(&self.d_inst)
    }
}

#[derive(Clone)]
pub struct DecalInstance {
    pub decal: Option<Arc<SmallD>>,
    pub pos: [Vf2d; 4],
    pub uv: [Vf2d; 4],
    pub w: [f32; 4],
    pub tint: [Pixel; 4],
}

impl Default for DecalInstance {
    fn default() -> Self {
        Self {
            decal: None,
            pos: [Vf2d::from((0.0, 0.0)); 4],
            uv: [
                Vf2d::from((0.0, 0.0)),
                Vf2d::from((0.0, 1.0)),
                Vf2d::from((1.0, 1.0)),
                Vf2d::from((1.0, 0.0)),
            ],
            w: [1.0; 4],
            tint: [Pixel::rgb(255, 255, 255); 4],
        }
    }
}

impl DecalInstance {
    fn get(&self) -> &Arc<SmallD> {
        self.decal.as_ref().unwrap()
    }
}

#[derive(Clone)]
pub struct DecalTriangleInstance {
    pub decal: Decal,
    pub points: [Vf2d; 3],
    pub texture: [Vf2d; 3],
    pub colours: [Pixel; 3],
}

impl DecalTriangleInstance {
    pub fn new() -> Self {
        Self {
            decal: Decal::empty(),
            points: [Vf2d::from((0.0, 0.0)); 3],
            texture: [Vf2d::from((0.0, 0.0)); 3],
            colours: [Pixel::rgb(255, 255, 255); 3],
        }
    }
}
