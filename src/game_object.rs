use crate::camera::Camera;
use crate::geometry;
use crate::geometry::{Mesh, Triangle};
use crate::math_3d::{Rotor3, Vector3};
use crate::olc::{Decal, OLCEngine, Pixel, Sprite};
use crate::transform::Transform3;
use crate::{math_3d, Game};
use num_traits::Num;
use std::collections::HashMap;
use std::sync::mpsc::channel;

pub struct GameObject {
    pub transform: Transform3,
    pub sprite: Option<Sprite>,
    pub mesh: Option<Mesh>,
    pub active: bool,
    pub uid: i32,
}

pub struct GameObjectCollection {
    pub game_objects: HashMap<i32, GameObject>,
    pub current_index: i32,
}

impl Default for GameObject {
    fn default() -> Self {
        Self {
            transform: Transform3 {
                pos: Vector3::default(),
                rot: Rotor3::default(),
            },
            sprite: None,
            mesh: None,
            active: false,
            uid: -1,
        }
    }
}

impl Default for GameObjectCollection {
    fn default() -> Self {
        GameObjectCollection {
            game_objects: HashMap::new(),
            current_index: 0,
        }
    }
}

impl GameObjectCollection {
    pub fn add(&mut self, mut obj: GameObject) {
        self.current_index += 1;
        obj.uid = self.current_index;
        self.game_objects.insert(self.current_index, obj);
    }

    pub fn contains(&self, index: i32) -> bool {
        self.game_objects.contains_key(&index)
    }
}

impl GameObject {
    pub fn draw(&self, layer: u32, camera: Camera, engine: &mut OLCEngine) {
        if self.sprite.is_none() {
            return;
        }
        let mut w = self.sprite.as_ref().unwrap().width as f32;
        let mut h = self.sprite.as_ref().unwrap().height as f32;

        let mut screen_transform = Transform3 {
            pos: camera.inv_camera_rot * // Invert the rotation
                (self.transform.pos - camera.transform.pos), // Invert the Translation
            rot: self.transform.rot,
        };

        if screen_transform.pos.z < 0.1 {
            return;
        }

        screen_transform.pos.x =
            (screen_transform.pos.x * camera.aspect / camera.fov) / screen_transform.pos.z;
        screen_transform.pos.y = (screen_transform.pos.y / camera.fov) / screen_transform.pos.z;

        screen_transform.pos.x += 1.0;
        screen_transform.pos.y += 1.0;

        screen_transform.pos.x *= camera.h_w;
        screen_transform.pos.y *= camera.h_h;

        w /= screen_transform.pos.z;
        h /= screen_transform.pos.z;

        screen_transform.pos.x -= w / 2.0;
        screen_transform.pos.y -= h / 2.0;

        if screen_transform.pos.x > engine.pixels_w as f32
            || screen_transform.pos.x + w < 0.0
            || screen_transform.pos.y > engine.pixels_h as f32
            || screen_transform.pos.y + h < 0.0
        {
            return;
        }

        let (mut sx, mut ex, mut sy, mut ey) = (
            screen_transform.pos.x.min(engine.pixels_w as f32),
            (screen_transform.pos.x + w).max(0.0),
            (screen_transform.pos.y).min(engine.pixels_h as f32),
            (screen_transform.pos.y + h).max(0.0),
        );
        if ey > engine.pixels_h as f32 {
            ey = engine.pixels_h as f32
        }
        if ex > engine.pixels_w as f32 {
            ex = engine.pixels_w as f32
        }

        let d = 1.0 / screen_transform.pos.z as f64;
        let u_step = screen_transform.pos.x + w - screen_transform.pos.x;
        let v_step = screen_transform.pos.y + h - screen_transform.pos.y;
        let mut u = 0.0;
        let mut v = 0.0;
        for y in sy.floor() as usize..ey.ceil() as usize {
            u = 0.0;
            for x in sx.floor() as usize..ex.ceil() as usize {
                //
                if engine.check_depth_buffer(y * engine.pixels_w as usize + x) < d {
                    let pixel = self.sprite.as_ref().unwrap().sample(u, v);
                    if pixel.a() == 255 {
                        engine.draw(x as i32, y as i32, pixel);
                        engine.update_depth_buffer(y * engine.pixels_w as usize + x, d);
                    } else if pixel.a() < 255 {
                        engine.draw(
                            x as i32,
                            y as i32,
                            pixel
                                * engine
                                    .get_layer_ref(layer)
                                    .unwrap()
                                    .sprite
                                    .get_pixel(x as u32, y as u32),
                        );
                    }
                }
                u = ((screen_transform.pos.x + w - x as f32) / u_step).clamp(0.0, 1.0);
            }
            v = ((screen_transform.pos.y + h - y as f32) / v_step).clamp(0.0, 1.0);
        }
        /*

        If we decide to go OpenGL and use Hardware Accelerated vertices for the level,
         this is probably the correct thing to use

        screen_transform.pos.y = engine.pixels_h as f32 - screen_transform.pos.y - h;

        engine.draw_warped_decal(environment.obj_decal.get(),
                                 &[(screen_transform.pos.x, screen_transform.pos.y).into(),
                                     (screen_transform.pos.x, screen_transform.pos.y + h).into(),
                                     (screen_transform.pos.x + w, screen_transform.pos.y + h).into(),
                                     (screen_transform.pos.x + w, screen_transform.pos.y ).into(),]
        );*/
    }

    pub fn render(&self, environment: &Game, engine: &mut OLCEngine) {
        let mut triangles: Vec<Triangle> = vec![];
        if let Some(mesh) = &self.mesh {
            //Render Triangles, Raster Triangles
            /*for tri in mesh.tris.iter() {
                triangles.append(&mut render_tri_ga(environment.camera,
                                                    tri, &self.transform));
            }
            if !triangles.is_empty() {
                if let Some(texture) = self.sprite.as_ref() {
                    engine.render_gl_tris(mesh.tris.as_slice(),
                                          texture.as_texture());
                } else {
                    engine.render_gl_tris(mesh.tris.as_slice(),
                                          0);
                }
            }*/
            /*for tri in mesh.tris.iter() {
                    triangles.append(&mut render_tri_ga(environment.camera,
                                      tri, &self.transform));
            }
            raster_triangles(engine,
                             triangles,
                             environment,
                             self.sprite.as_ref());*/
        }
    }

    fn thread_tri() {}

    pub fn check_collision(&self, obj: &GameObject) -> (bool, Vector3) {
        (false, Vector3::default())
    }

    pub fn get_mesh(&self) -> &Mesh {
        self.mesh.as_ref().unwrap()
    }
    pub fn get_sprite(&self) -> &Sprite {
        self.sprite.as_ref().unwrap()
    }

    pub fn new(
        pos: Vector3,
        rot: Rotor3,
        sprite: Option<Sprite>,
        mesh: Option<Mesh>,
    ) -> GameObject {
        let mut obj = GameObject::default();
        obj.transform = Transform3 { pos, rot };
        obj.sprite = sprite;
        obj.mesh = mesh;
        obj.active = true;
        obj
    }

    pub fn new_inactive(
        pos: Vector3,
        rot: Rotor3,
        sprite: Option<Sprite>,
        mesh: Option<Mesh>,
    ) -> GameObject {
        let mut obj = GameObject::default();
        obj.transform = Transform3 { pos, rot };
        obj.sprite = sprite;
        obj.mesh = mesh;
        obj
    }
}
