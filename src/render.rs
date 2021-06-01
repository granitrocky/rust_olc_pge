use crate::geometry::*;
use crate::math_3d::*;
use crate::math_4d::*;
use crate::Game;
use crate::transform::{Transform3, Transform4};
use crate::olc::{Pixel, OLCEngine, Sprite};
use std::cmp::Ordering::Equal;
use num_traits::Num;
use crate::camera::Camera;

pub fn render_tri_ga(camera: Camera, tri: &Triangle,
                     transform: &Transform3 ) -> Vec<Triangle>{

    //Apply the vertex's transformation:
    let p1: Vector3 = transform.rot.rotate_vector(&tri.p[0]) + transform.pos;
    let p2: Vector3 = transform.rot.rotate_vector(&tri.p[1]) + transform.pos;
    let p3: Vector3 = transform.rot.rotate_vector(&tri.p[2]) + transform.pos;

    let mut scene_tri: Triangle = ( p1, p2, p3).into();
    let mut return_tris: Vec<Triangle> = vec![];
    let normal = (scene_tri.p[1] - scene_tri.p[0])
        .cross(&(scene_tri.p[2] - scene_tri.p[0])).normal();
    let light_dir = (camera.transform.pos + (camera.transform.rot * (0.0, 0.0, 1.0))).normal();
    scene_tri.l = normal.dot(&light_dir);


    if normal.dot(&( scene_tri.p[0] - (camera.transform.pos))) < 0.0 {

        let to_screen_space = |t: &Triangle| -> Triangle{
            let mut ret_tri = Triangle::default();
            ret_tri.l = t.l;
            ret_tri.c = t.c;
            for (i, v) in t.p.iter().enumerate() {
                ret_tri.p[i] = (*v - camera.transform.pos) * // Invert the Translation first
                 camera.inv_camera_rot; // Then invert the rotation

            }
            ret_tri
        };

        let mut screen_tri: Triangle = to_screen_space(&scene_tri);

        screen_tri.t = tri.t;
        let mut clipped_tris = clip_against_plane(&screen_tri, &Plane{
            p: (0.0, 0.0, camera.clip_near).into(),
            n: (0.0, 0.0, 1.0).into(),
        });

        for clipped in clipped_tris.iter_mut() {
            for (i, t_p) in clipped.p.iter_mut().enumerate() {
                //Scale the triangle by its distance from the camera and apply fov
                t_p.z = 1.0 / t_p.z;
                t_p.x = (t_p.x / camera.aspect / camera.fov) * t_p.z;
                t_p.y = (t_p.y / camera.fov) * t_p.z;

                clipped.t[i].u *= t_p.z;
                clipped.t[i].v *= t_p.z;
                clipped.t[i].w = t_p.z;


                t_p.x = (t_p.x + 1.0) * camera.h_w;
                t_p.y = (t_p.y + 1.0) * camera.h_h;
            }

            //clipped.c = Pixel::WHITE;
            clipped.c = tri.c;
            return_tris.push(*clipped);
       }
    }
    return_tris
}

pub fn raster_triangles(engine: &mut OLCEngine,
                        raster_tris: Vec<Triangle>,
                        environment: &Game, texture: Option<&Sprite>){

    for tri in raster_tris.iter(){
        let mut new_triangles = 1;
        let mut vec_triangles: Vec<Triangle> = vec![];
        vec_triangles.push(*tri);
        for p in 0..4{
            while new_triangles > 0{
                let tri_test = vec_triangles.pop().unwrap();
                new_triangles -= 1;
                let mut clipped: Vec<Triangle> = vec![];
                match p{
                    0 => {
                        clipped.append(
                        &mut clip_against_plane(&tri_test, &Plane{
                            p: (0.0, 0.0, 0.0).into(), n: (0.0, 1.0, 0.0).into()
                        }));}
                    1 => {
                        clipped.append(
                        &mut clip_against_plane(&tri_test, &Plane{
                            p: (0.0, engine.pixels_h as f32, 0.0).into(), n: (0.0, -1.0, 0.0).into()
                        }));}
                    2 => {
                        clipped.append(
                        &mut clip_against_plane(&tri_test, &Plane{
                            p: (0.0, 0.0, 0.0).into(), n: (1.0, 0.0, 0.0).into()
                        }));}
                    3 => {
                        clipped.append(
                        &mut clip_against_plane(&tri_test, &Plane{
                            p: (engine.pixels_w as f32 , 0.0, 0.0).into(), n: (-1.0, 0.0, 0.0).into()
                        }));}
                    _ => {}
                }
                for nt in clipped.iter(){
                    vec_triangles.insert(0, *nt );
                }
            }
            new_triangles = vec_triangles.len();
        }

        for t in vec_triangles.iter_mut() {/*
            engine.fill_triangle(
                (t.p[0].x, t.p[0].y).into(),
                (t.p[1].x, t.p[1].y).into(),
                (t.p[2].x, t.p[2].y).into(),
                t.c * t.l,
            );*/

            engine.texture_triangle(
                (t.p[0].x, t.p[0].y).into(),
                (t.p[1].x, t.p[1].y).into(),
                (t.p[2].x, t.p[2].y).into(),
                t.t[0], t.t[1], t.t[2],
                texture,
                t.c * t.l
            );
        }
    }
}

pub fn clip_against_plane(tri: &Triangle, plane: &Plane) -> Vec<Triangle>{
    let mut v: Vec<Triangle> = vec![];
    let np = plane.n.normal();
    let dist = |p: Vector3| -> f32{
         np.dot(&p) - np.dot(&plane.p)
    };
    let (mut i_count, mut o_count) = (0,0);
    let zv = Vector3::default();
    let zuv: UV = (0.0, 0.0).into();
    let mut inside_points = [&zv; 3];
    let mut inside_tex = [&zuv; 3];
    let mut outside_points = [&zv; 3];
    let mut outside_tex = [&zuv; 3];
    let (d1, d2, d3) = (dist(tri.p[0]), dist(tri.p[1]), dist(tri.p[2]));

    if d1 >= 0.0{ inside_points[i_count] = &tri.p[0]; inside_tex[i_count] = &tri.t[0]; i_count += 1;}
    else{ outside_points[o_count] = &tri.p[0]; outside_tex[o_count] = &tri.t[0]; o_count += 1; }

    if d2 >= 0.0{ inside_points[i_count] = &tri.p[1]; inside_tex[i_count] = &tri.t[1]; i_count += 1;}
    else{ outside_points[o_count] = &tri.p[1]; outside_tex[o_count] = &tri.t[1]; o_count += 1; }

    if d3 >= 0.0{ inside_points[i_count] = &tri.p[2]; inside_tex[i_count] = &tri.t[2]; i_count += 1;}
    else{ outside_points[o_count] = &tri.p[2]; outside_tex[o_count] = &tri.t[2]; o_count += 1; }

    if i_count == 3{
        v.push(*tri);
    }
    if i_count == 1 && o_count == 2{
        let t = Triangle{
            p: [
                *inside_points[0],
                intersect_plane(plane, inside_points[0], outside_points[0]),
                intersect_plane(plane, inside_points[0], outside_points[1]),
            ],
            t: [*inside_tex[0],
                *inside_tex[0] + ((*outside_tex[0] - *inside_tex[0])
                    * intersect_plane_percent(plane,
                                              inside_points[0],
                                              outside_points[0])),

                *inside_tex[0] + ((*outside_tex[1] - *inside_tex[0])
                    * intersect_plane_percent(plane,
                                              inside_points[0],
                                              outside_points[1])),
            ],
            l: tri.l,
            c: tri.c,
        };
        v.push(t);
    }
    if i_count == 2 && o_count == 1{
        let t1 = Triangle{
            p: [
                *inside_points[0],
                *inside_points[1],
                intersect_plane(plane, inside_points[0], outside_points[0]),
            ],
            t: [*inside_tex[0],
                *inside_tex[1],
                *inside_tex[0] + ((*outside_tex[0] - *inside_tex[0])
                    * intersect_plane_percent(plane,
                                              inside_points[0],
                                              outside_points[0])),
            ],
            l: tri.l,
            c: tri.c,
        };
        let t2 = Triangle{
            p: [
                *inside_points[1],
                t1.p[2],
                intersect_plane(plane, inside_points[1], outside_points[0]),
            ],
            t: [*inside_tex[1],
                t1.t[2],
                *inside_tex[1] + ((*outside_tex[0] - *inside_tex[1])
                    * intersect_plane_percent(plane,
                                              inside_points[1],
                                              outside_points[0])),
            ],
            l: tri.l,
            c: tri.c,
        };
        v.push(t1);
        v.push(t2);
    }
    v

}
