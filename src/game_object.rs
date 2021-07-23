#![allow(clippy::many_single_char_names)]
use super::{
    camera::Camera,
    engine::OLCEngine,
    geometry::{Mesh, MeshType, Triangle, Vertex},
    layer::{LayerMask, Mask},
    math_3d::{Rotor3, Vector3},
    sprite::Sprite,
    transform::Transform3,
};
use std::collections::HashMap;

pub struct GameObject {
    pub transform: Transform3,
    pub sprite: Option<Sprite>,
    pub meshes: Vec<Mesh>,
    pub children: Vec<GameObject>,
    pub active: bool,
    pub uid: i32,
    pub layer_mask: Mask,
}

impl Clone for GameObject {
    fn clone(&self) -> Self {
        Self {
            transform: self.transform,
            sprite: self.sprite.clone(),
            meshes: self.meshes.clone(),
            children: self.children.clone(),
            active: self.active,
            uid: -1,
            layer_mask: self.layer_mask,
        }
    }
}

pub struct GameObjectCollection {
    pub game_objects: HashMap<i32, GameObject>,
    pub current_index: i32,
}

impl Default for GameObject {
    fn default() -> Self {
        Self {
            transform: Transform3::default(),
            sprite: None,
            meshes: vec![],
            active: false,
            children: vec![],
            layer_mask: Mask::D3,
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
    pub fn get_vertices_and_indices(
        &self,
        mut vert_count: u32,
        mut index_count: u32,
    ) -> (Vec<Vertex>, Vec<u32>, u32, u32) {
        let mut vert_slice: Vec<Vertex> = vec![];
        let mut ind_slice: Vec<u32> = vec![];
        for mesh in &self.meshes {
            //Non Indexed models don't store what we need, so we'll have to extract every vertex and each one gets their own index.
            match &mesh.mesh_type {
                MeshType::Indexed(verts, indices) => {
                    ind_slice.extend(
                        mesh.buffer_indices
                            .iter()
                            .map(|i| i + vert_count)
                            .collect::<Vec<_>>(),
                    );
                    vert_slice.extend(
                        verts
                            .iter()
                            .map(|v| Vertex {
                                position: ((v.position * self.transform.scale)
                                    * self.transform.rot)
                                    + self.transform.pos,
                                normal: v.normal,
                                tex_coords: v.tex_coords,
                                color: v.color,
                            })
                            .collect::<Vec<_>>(),
                    );
                    index_count += indices.len() as u32;
                    vert_count += verts.len() as u32;
                }
                MeshType::NonIndexed(tris) => {
                    let new_verts: Vec<Vertex> = tris
                        .iter()
                        .flat_map(|t| {
                            t.as_vertex_vec()
                                .iter()
                                .map(|v| Vertex {
                                    position: ((v.position * self.transform.scale)
                                        * self.transform.rot)
                                        + self.transform.pos,
                                    normal: v.normal,
                                    tex_coords: v.tex_coords,
                                    color: v.color,
                                })
                                .collect::<Vec<Vertex>>()
                        })
                        .collect();
                    ind_slice.extend(
                        mesh.buffer_indices
                            .iter()
                            .map(|i| i + vert_count)
                            .collect::<Vec<_>>(),
                    );
                    index_count += mesh.buffer_indices.len() as u32;
                    vert_count += new_verts.len() as u32;
                    vert_slice.extend(new_verts);
                }
            };
        }

        for child in &self.children {
            let (mut verts, mut i, vc, ic) =
                child.get_vertices_and_indices(vert_count, index_count);
            vert_count = vc;
            index_count = ic;
            ind_slice.extend(i);
            //vert_slice.extend(verts);
            vert_slice.extend::<Vec<Vertex>>(
                verts
                    .iter()
                    .map(|v| Vertex {
                        position: ((v.position * self.transform.scale) * self.transform.rot)
                            + self.transform.pos,
                        normal: v.normal,
                        tex_coords: v.tex_coords,
                        color: v.color,
                    })
                    .collect(),
            );
        }

        (vert_slice, ind_slice, vert_count, index_count)
    }

    pub fn get_indices(&self) -> Vec<u32> {
        let mut ind_slice: Vec<u32> = vec![];
        ind_slice.extend(
            self.meshes
                .iter()
                .flat_map(|m| m.buffer_indices.clone())
                .collect::<Vec<u32>>(),
        );

        for child in &self.children {
            ind_slice.extend(child.get_indices());
        }
        ind_slice
    }

    pub fn set_buffer_indices(&mut self, mut index_offset: u32) {
        for mesh in self.meshes.iter_mut() {
            let mut ind_slice: Vec<u32> = vec![];
            //Non Indexed models don't store what we need, so we'll have to extract every vertex and each one gets their own index.
            match &mesh.mesh_type {
                MeshType::Indexed(verts, indices) => {
                    ind_slice.extend(indices.iter().map(|i| i + index_offset));
                    index_offset += verts.len() as u32;
                }
                MeshType::NonIndexed(tris) => {
                    ind_slice.extend(
                        tris.iter()
                            .flat_map(|t| [t.v[0], t.v[1], t.v[2]])
                            .enumerate()
                            .map(|(i, _)| i as u32 + index_offset),
                    );
                    index_offset += (tris.len() * 3) as u32;
                }
            };
            mesh.buffer_offset = index_offset;
            mesh.buffer_indices = ind_slice;
        }

        for child in self.children.iter_mut() {
            child.set_buffer_indices(index_offset);
        }
    }

    pub fn get_triangles(&self) -> Vec<Triangle> {
        self.meshes
            .iter()
            .flat_map(|mesh| {
                match &mesh.mesh_type {
                    MeshType::NonIndexed(tris) => tris.clone(),
                    //Indexed models don't store triangles.
                    MeshType::Indexed(verts, indices) => vec![],
                }
            })
            .collect()
    }

    pub fn get_transformed_triangles(&self) -> Vec<Triangle> {
        self.meshes
            .iter()
            .flat_map(|mesh| {
                match &mesh.mesh_type {
                    MeshType::NonIndexed(tris) => tris
                        .iter()
                        .enumerate()
                        .map(|(i, t)| {
                            let mut t2 = *t;
                            t2.v[0] = Vertex {
                                position: (t.v[0].position - self.transform.pos)
                                    * self.transform.rot,
                                tex_coords: t.v[0].tex_coords,
                                normal: t.v[0].normal * self.transform.rot,
                                color: t.v[0].color,
                            };
                            t2.v[1] = Vertex {
                                position: (t.v[1].position - self.transform.pos)
                                    * self.transform.rot,
                                tex_coords: t.v[1].tex_coords,
                                normal: t.v[1].normal * self.transform.rot,
                                color: t.v[1].color,
                            };
                            t2.v[2] = Vertex {
                                position: (t.v[2].position - self.transform.pos)
                                    * self.transform.rot,
                                tex_coords: t.v[2].tex_coords,
                                normal: t.v[2].normal * self.transform.rot,
                                color: t.v[2].color,
                            };
                            t2
                        })
                        .collect(),
                    //Indexed models don't store triangles.
                    MeshType::Indexed(verts, indices) => vec![],
                }
            })
            .collect()
    }

    pub fn apply_transform(&mut self) {
        for mesh in &mut self.meshes {
            mesh.apply_transform(self.transform);
        }

        for child in self.children.iter_mut() {
            child.apply_transform();
        }
    }

    pub fn draw<D: crate::olc::OlcData>(
        &self,
        layer: u32,
        camera: Camera,
        engine: &mut OLCEngine<D>,
    ) {
        if self.sprite.is_none() {
            return;
        }
        let mut w = self.sprite.as_ref().unwrap().width as f32;
        let mut h = self.sprite.as_ref().unwrap().height as f32;

        let mut screen_transform = Transform3 {
            pos: camera.inv_camera_rot * // Invert the rotation
                (self.transform.pos - camera.transform.pos), // Invert the Translation
            rot: self.transform.rot,
            ..Default::default()
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
                    } else {
                        engine.draw(
                            x as i32,
                            y as i32,
                            pixel
                                * engine
                                    .get_image_layer_ref(layer)
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

    fn thread_tri() {}

    pub fn check_collision(&self, obj: &GameObject) -> (bool, Vector3) {
        (false, Vector3::default())
    }

    pub fn get_meshes(&self) -> &[Mesh] {
        self.meshes.as_slice()
    }
    pub fn get_sprite(&self) -> &Sprite {
        self.sprite.as_ref().unwrap()
    }

    pub fn new(transform: Transform3, sprite: Option<Sprite>, meshes: Vec<Mesh>) -> GameObject {
        GameObject {
            transform,
            sprite,
            meshes,
            active: true,
            children: vec![],
            layer_mask: Mask::D3,
            uid: -1,
        }
    }

    pub fn new_inactive(
        pos: Vector3,
        rot: Rotor3,
        sprite: Option<Sprite>,
        meshes: Vec<Mesh>,
    ) -> GameObject {
        GameObject {
            transform: Transform3 {
                rot,
                pos,
                scale: Vector3::one(),
            },
            sprite,
            meshes,
            layer_mask: Mask::D3,
            active: false,
            children: vec![],
            uid: -1,
        }
    }
}

impl LayerMask for GameObject {
    type BitFlags = Mask;

    fn in_layer_mask(&self, mask: Self::BitFlags) -> bool {
        mask.intersects(self.layer_mask)
    }
    fn set_layer_mask(&mut self, mask: Self::BitFlags) {
        self.layer_mask = mask;
    }
    fn add_layer(&mut self, mask: Self::BitFlags) {
        self.layer_mask.insert(mask);
    }
    fn remove_layer(&mut self, mask: Self::BitFlags) {
        self.layer_mask.remove(mask);
    }
    fn reset_mask(&mut self) {
        self.layer_mask = Mask::empty();
    }
}
