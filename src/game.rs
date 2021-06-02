
use crate::geometry;

use crate::olc::*;
use crate::math_3d::*;
use crate::math_4d::*;
use crate::math_nd::*;
use number_traits::{tanf32, Num, Round};
use std::io::BufRead;
use std::cmp::Ordering::Equal;
use crate::geometry::*;
use crate::render::*;
use crate::transform::{Transform4, Transform3};
use crate::render::raster_triangles;
use crate::game_object::{GameObject, GameObjectCollection};
use std::collections::HashMap;
use crate::camera::Camera;

impl From<Vector3> for Vector4{
    fn from(v: Vector3) -> Self {
        Vector4{
            x: v.x, y: v.y, z: v.z, w: 0.0
        }
    }
}
impl From<Vector4> for Vector3{
    fn from(v: Vector4) -> Self {
        Vector3 {
            x: v.x,
            y: v.y,
            z: v.z
        }
    }
}
impl From<Rotor4> for Rotor3{
    fn from(r: Rotor4) -> Self {
        Rotor3::from_bivector(&(r.b12, r.b13, r.b23).into())
    }
}
impl Rotor4{
    fn as_rotor3(&self) -> Rotor3 {
        Rotor3::from_bivector(&(self.b12, self.b13, self.b23).into())
    }
}


pub struct Game{
    pub level: GameObject,
    pub gun: GameObject,
    pub player: Vector3,
    pub camera: Camera,
    pub enemies: GameObjectCollection,
    pub player_height: f32,
    pub player_velocity: Vector3,
    pub camera_slerp: f32,
    pub meshes: i32,
    pub wireframe: bool,
    pub speed: f32,
    pub turn_speed: f32,
    pub renderer: i8,
    pub prev_mouse_position: Vf2d,
    pub decal: Decal,
    pub obj_decal: Decal,
    pub other_layer: u32,
    pub ui_layer: u32,
    pub transparency: Pixel,
    pub lock_mouse: bool,
    pub gravity: f32,
    pub crosshair: Decal,
}


impl Game{
    pub fn new() -> Self{
        Game{
            level: GameObject::new(
                (0.0, 0.0, 0.0).into(),Rotor3::default(),
                None,//Sprite::load_from_file::<PNGLoader>("wall_bricks_basic.png"),
                Mesh::load_from_file("models/teapot.obj", false),
            ),
            gun:  GameObject::new(
                (0.0, 0.0, 0.0).into(),Rotor3::default(),
                None,
                Mesh::load_from_file("models/Shotgun.obj", true),
            ),
            camera: Camera{
                transform: (Vector3::default(), Rotor3::default()).into(),
                h_w: 0.0,
                h_h: 0.0,
                clip_near: 0.1,
                clip_far: 5000.0,
                inv_camera_rot: Rotor3::default(),
                aspect: 0.0,
                fov: 90.0,
            },
            player_velocity: (0.0, 0.0, 0.0).into(),
            player: (2.0, 0.5, -3.0).into(),
            player_height: 1.0,
            camera_slerp: 0.0,
            meshes: 2,
            wireframe: false,
            speed: 4.0,
            turn_speed: 4.0,
            crosshair: Decal::empty(),
            renderer: 0,
            prev_mouse_position: Vf2d::new(0.0, 0.0),
            decal: Decal::empty(),
            obj_decal: Decal::empty(),
            other_layer: 0,
            ui_layer: 0,
            transparency: Pixel::rgb(153,217,234),
            lock_mouse: true,
            gravity: -4.8,
            enemies: GameObjectCollection::default(),
        }
    }

    fn screen_to_world(&self, screen_position: Vf2d, distance: f32) -> Vector3 {
        let mut t_p: Vector3 = (screen_position.x, screen_position.y, 1.0).into();
        t_p.x /= self.camera.h_w;
        t_p.y /= self.camera.h_h;
        t_p.x -= 1.0;
        t_p.y -= 1.0;

        t_p.x *= 1.0 / (self.camera.aspect / self.camera.fov) * t_p.z;
        t_p.y *= self.camera.fov * t_p.z;

        self.camera.transform.pos
            + (self.camera.transform.rot * (t_p * distance))
        //this returns the point AT the world coords
        //You will need to multiply by a float to move it forward.
    }

    fn world_to_screen(&self, world_point: Vector3) -> Vi2d {
        let mut t_p = self.camera.inv_camera_rot * (world_point - self.camera.transform.pos);
        //Scale the triangle by its distance from the camera and apply fov
        if t_p.z < 0.1 { return (-1,-1).into(); }
        t_p.x = (t_p.x * self.camera.aspect / self.camera.fov) / t_p.z;
        t_p.y = (t_p.y / self.camera.fov) / t_p.z;

        t_p.x += 1.0;
        t_p.y += 1.0;
        t_p.x *= self.camera.h_w;
        t_p.y *= self.camera.h_h;

        (t_p.x.floor() as i32, t_p.y.floor() as i32).into()
    }
    fn ray_cast_all<'a>(&self, start_point: Vector3, end_point: Vector3, objects_to_check: &[&'a GameObject])
                        -> Vec<&'a GameObject>{
        let mut return_objects: Vec<&'a GameObject> = vec![];
        for obj in objects_to_check.iter(){
            if let Some(mesh) = &obj.mesh{
                if mesh.get_collision_point(start_point, end_point - start_point).unwrap_or_default()
                    != Vector3::default()
                {
                    return_objects.push(*obj);
                }
            }
        }
        return_objects
    }

    fn ray_cast<'a>(&self, start_point: Vector3, end_point: Vector3, objects_to_check: &[&'a GameObject])
                    -> (Option<&'a GameObject>, Vector3){
        let mut return_object: (Option<&'a GameObject>, Vector3) = (None, Vector3::default());
        let mut closest = Vector3::max();
        for obj in objects_to_check.iter(){
            //First check for geometry. If it has a mesh, ignore the sprite.
            if let Some(mesh) = &obj.mesh{
                if let Ok(collision) = mesh.get_collision_point(start_point, end_point - start_point)
                {
                    if  (collision - start_point).length()
                        < (closest - start_point).length() {
                        closest = collision;
                        return_object = (Some(*obj), closest);
                    }
                }
            }// If we don't have a mesh, then this sprite is in the world.
            // We may need to have a flag for "Drawn"
            else if let Some(sprite) = &obj.sprite{
                let intersection = geometry::intersect_plane_with_percent(&Plane{
                    p: obj.transform.pos,
                    n: (obj.transform.pos - self.camera.transform.pos).normal(),
                }, &start_point, &end_point);
                if intersection.1 <= 1.0 && intersection.1 >= 0.0
                    && (intersection.0.x - obj.transform.pos.x).abs() < 0.5
                    && (intersection.0.y - obj.transform.pos.y).abs() < 0.5
                    && (intersection.0 - start_point).length() < (closest - start_point).length()
                {
                    closest = intersection.0;
                    return_object = (Some(*obj), closest);
                }
            }
        }
        return_object
    }

}

impl Olc for Game{
    fn on_engine_start(&mut self, engine: &mut OLCEngine) -> bool {

        true
    }

    fn on_engine_update(&mut self, engine: &mut OLCEngine, elapsedTime: f32) -> bool {
        engine.clear_depth_buffer();

        !engine.get_key(Key::ESCAPE).pressed
    }
    fn on_engine_destroy(&mut self) -> bool {
        true
    }
}
