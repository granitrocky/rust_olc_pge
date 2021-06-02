#![feature(try_trait)]
#![allow(dead_code)]
#![allow(unused)]
#![feature(once_cell)]
mod math_3d;
mod math_4d;
mod olc;
//mod math_nd;
mod camera;
mod game_object;
mod geometry;
mod transform;

use crate::camera::Camera;
use crate::game_object::*;
use crate::geometry::*;
use crate::transform::*;
use math_3d::*;
use math_4d::*;
use olc::*;
use rand::Rng;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
enum MagicType {
    /// Corresponds to a physical weight in grams.
    Raw,
    /// Fire Magic
    Fire,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Point {
    position: [f32; 3],
    /// MagicType
    magic: MagicType,
    /// Amount of Magic of the given type
    value: f32,
    /// Radius of sphere of influence in `cm`
    density: f32,
    /// The rate at which this point loses magic as
    /// a rate in `percentage / second`
    emission: f32,
    /// The rate at which this point grabs nearby
    /// magic in terms of `value / second`
    absorption: f32,
}
unsafe impl bytemuck::Zeroable for Point {}
unsafe impl bytemuck::Pod for Point {}

impl Default for Point {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            magic: MagicType::Raw,
            value: 0.0,
            density: 0.0,
            emission: 0.0,
            absorption: 0.0,
        }
    }
}

impl Point {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Point>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Uint32,
                },
                wgpu::VertexAttribute {
                    offset: (mem::size_of::<[f32; 3]>() + mem::size_of::<u32>())
                        as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: (mem::size_of::<[f32; 3]>()
                        + mem::size_of::<u32>()
                        + mem::size_of::<f32>()) as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: (mem::size_of::<[f32; 3]>()
                        + mem::size_of::<u32>()
                        + mem::size_of::<f32>() * 2)
                        as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: (mem::size_of::<[f32; 3]>()
                        + mem::size_of::<u32>()
                        + mem::size_of::<f32>() * 3)
                        as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}

struct Object {
    points: Vec<Point>,
    transform: transform::Transform3,
}

pub struct Game {
    player: Vector3,
    ui_layer: u32,
    player_height: f32,
    player_velocity: Vector3,
    speed: f32,
    turn_speed: f32,
    prev_mouse_position: Vf2d,
    vertex_buffer: Option<wgpu::Buffer>,
    camera_buffer: Option<wgpu::Buffer>,
    render_pipeline: Option<wgpu::RenderPipeline>,
    persp_shader: Option<wgpu::ShaderModule>,
    cam_uniform_group: Option<wgpu::BindGroup>,
    camera_velocity: Vf2d,
}

impl Game {
    fn new() -> Self {
        Game {
            player_velocity: (0.0, 0.0, 0.0).into(),
            player: (0.0, 0.0, 1.0).into(),
            player_height: 0.5,
            speed: 2.0,
            turn_speed: 100.0,
            prev_mouse_position: Vf2d::new(0.0, 0.0),
            vertex_buffer: None,
            camera_buffer: None,
            render_pipeline: None,
            persp_shader: None,
            cam_uniform_group: None,
            camera_velocity: (0.0, 0.0).into(),
            ui_layer: 0,
        }
    }
}

impl Olc for Game {
    fn on_engine_start(&mut self, engine: &mut OLCEngine) -> bool {
        let p = Point {
            position: [0.0; 3],
            magic: MagicType::Fire,
            emission: 1.0,
            absorption: 2.0,
            value: 10.0,
            density: 50.0,
        };
        engine.renderer.points = vec![p; 4096];
        for p in engine.renderer.points.iter_mut() {
            (*p).position = [
                rand::thread_rng().gen_range(-0.2, 0.1),
                rand::thread_rng().gen_range(0.0, 1.0),
                rand::thread_rng().gen_range(-0.2, 0.1),
            ];
            (*p).value = rand::thread_rng().gen_range(50.0, 1500.0);
        }
        //engine.renderer.setup_3D_pipeline();
        engine.camera = Camera {
            transform: ((0.0, 0.0, 0.0).into(), Rotor3::default()).into(),
            h_w: engine.pixels_w as f32 * 0.5,
            h_h: engine.pixels_h as f32 * 0.5,
            clip_near: 0.1,
            clip_far: 1000.0,
            inv_camera_rot: Rotor3::default(),
            aspect: engine.pixels_w as f32 / engine.pixels_h as f32,
            fov: 90.0 / 360.0 * std::f32::consts::PI,
            mat: camera::RawMat::default(),
        };
        engine.camera.mat.view_proj = engine.camera.build_view_projection_matrix().into();

        self.ui_layer = engine.add_layer();
        engine.set_layer_visible(self.ui_layer, true);
        /*self.obj_decal = Decal::create(
            Sprite::load_from_file::<BMPLoader>("stickdude.bmp"), &mut engine.renderer);
        self.decal = Decal::create(
            Sprite::load_from_file::<PNGLoader>("dot.png"), &mut engine.renderer);*/
        engine.renderer.add_mesh(geometry::Primitives::cube());

        //This is a hack for wasm. For some reason the pipeline goes away the other way.
        engine.window.set_cursor_grab(true);

        self.fix_binds(engine);
        true
    }

    fn on_engine_update(&mut self, engine: &mut OLCEngine, elapsed_time: f64) -> bool {
        let elapsed_time = elapsed_time as f32;
        let center: Vf2d = (engine.pixels_w as f32 / 2.0, engine.pixels_h as f32 / 2.0).into();

        /*if engine.get_key(Key::H).pressed{ engine.hide_mouse()}
        if engine.get_key(Key::P).pressed{ engine.show_mouse()}*/

        //self.player_velocity +=
        // /*self.camera_rot * */Vector3::new(0.0, -9.6 * elapsed_time, 0.0);
        if engine.get_key(Key::D).held {
            let direction = engine.camera.transform.rot * Vector3::new(1.0, 0.0, 0.0);
            self.player_velocity += direction * elapsed_time * self.speed;
        }
        if engine.get_key(Key::A).held {
            let direction = engine.camera.transform.rot * Vector3::new(1.0, 0.0, 0.0);
            self.player_velocity -= direction * elapsed_time * self.speed;
        }

        if engine.get_key(Key::W).held {
            let direction = engine.camera.transform.rot * Vector3::new(0.0, 0.0, 1.0);
            self.player_velocity += direction * elapsed_time * self.speed;
        }
        if engine.get_key(Key::S).held {
            let direction = engine.camera.transform.rot * Vector3::new(0.0, 0.0, 1.0);
            self.player_velocity -= direction * elapsed_time * self.speed;
        }

        /*if let Ok(direction) =
            self.level.get_mesh().get_collision_correction(self.player,
                                                           self.player_velocity, 5.0)
        {
            self.player_velocity = direction;// * elapsedTime * self.speed;
        }*/
        self.player += self.player_velocity;
        engine.camera.transform.pos = self.player + Vector3::new(0.0, self.player_height, 0.0);

        self.player_velocity *= 0.4;

        let mouse = engine.get_mouse_pos();
        let v = mouse - self.prev_mouse_position;
        self.camera_velocity.x = -v.x;
        self.camera_velocity.y = v.y;
        engine.set_mouse_pos(engine.pixels_w as f32 / 2.0, engine.pixels_h as f32 / 2.0);
        self.prev_mouse_position = engine.get_mouse_pos();

        if engine.get_mouse(Mouse::LEFT).held {
            engine.set_draw_target(self.ui_layer);
            engine.draw(mouse.x as i32, mouse.y as i32, Pixel::RED);
            engine.set_layer_update(self.ui_layer, true);
            engine.reset_draw_target();
        }

        let f = Vector3::forward();
        let l = Vector3::new(
            (self.camera_velocity.x / engine.pixel_width as f32)
                .abs()
                .min(self.turn_speed)
                * self.camera_velocity.x.signum(),
            0.0,
            0.0,
        );
        let yaw = Rotor3::from_vectors(&f, &l);

        let u = Vector3::new(
            0.0,
            (self.camera_velocity.y / engine.pixel_height as f32)
                .abs()
                .min(self.turn_speed)
                * self.camera_velocity.y.signum(),
            0.0,
        );
        let pitch = Rotor3::from_vectors(&f, &u);

        engine.camera.transform.rot =
            (pitch * elapsed_time) * engine.camera.transform.rot * (yaw * elapsed_time);

        self.camera_velocity.x = (self.camera_velocity.x.abs() - (self.turn_speed)).max(0.0)
            * self.camera_velocity.x.signum();

        self.camera_velocity.y = (self.camera_velocity.y.abs() - (self.turn_speed)).max(0.0)
            * self.camera_velocity.y.signum();

        engine.camera.mat.view_proj = engine.camera.build_view_projection_matrix().into();
        engine.camera.mat.view_inv_proj = engine.camera.build_reverse_projection_matrix().into();

        engine.camera.mat.position = engine.camera.transform.pos.into();

        !engine.get_key(Key::Escape).pressed
    }
    fn on_engine_destroy(&mut self) -> bool {
        true
    }
}

impl Vector4 {
    fn as_vector3(&self) -> Vector3 {
        Vector3 {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }
}
impl Vector3 {
    fn as_vector4(&self) -> Vector4 {
        Vector4 {
            x: self.x,
            y: self.y,
            z: self.z,
            w: 0.0,
        }
    }
}

fn point_at(r: Rotor3, target: Vector3) -> Rotor3 {
    let forward = r * Vector3::new(0.0, 0.0, 1.0);
    Rotor3::from_vectors(&forward, &target)
}

fn main() {
    #[cfg(feature = "web-sys")]
    console_error_panic_hook::set_once();

    let mut game = Game::new();

    #[cfg(not(target_arch = "wasm32"))]
    futures::executor::block_on(olc::construct(
        game,
        "Test Game",
        1280,
        720,
        1,
        1,
        false,
        false,
    ));

    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(olc::construct(
        game,
        "Test Game",
        1280,
        720,
        1,
        1,
        false,
        false,
    ));
}

#[cfg(feature = "web-sys")]
mod wasm {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub fn run() {
        console_log::init_with_level(log::Level::Trace);
        super::main();
    }
}
trait FixBind {
    fn fix_binds(&mut self, engine: &mut OLCEngine);
}

impl FixBind for Game {
    fn fix_binds(&mut self, engine: &mut OLCEngine) {
        self.persp_shader = Some(engine.renderer.device.create_shader_module(
            &wgpu::ShaderModuleDescriptor{
                label: Some("persp_shader"),
                flags: wgpu::ShaderFlags::all(),
                source: wgpu::ShaderSource::Wgsl(include_str!("persp.wgsl").into()),
            }));
        let bind_group_layout =
            engine
                .renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });
        self.cam_uniform_group = Some(engine.renderer.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: None,
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: engine.renderer.camera_buffer.as_entire_binding(),
                }],
            },
        ));
        let pipeline_layout =
            engine
                .renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });
        self.render_pipeline = Some(engine.renderer.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &engine.renderer.base_3d_shader,
                    entry_point: "vs_main",
                    buffers: &[Point::desc()],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip, // 1.
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw, // 2.
                    cull_mode: None,
                    // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                    clamp_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &engine.renderer.base_3d_shader,
                    entry_point: "fs_main",
                    targets: &[wgpu::ColorTargetState {
                        format: engine.renderer.sc_desc.format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                operation: wgpu::BlendOperation::Add,
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            },
                            alpha: wgpu::BlendComponent {
                                operation: wgpu::BlendOperation::Add,
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::One,
                            },
                        }),
                        write_mask: wgpu::ColorWrite::ALL,
                    }],
                }),
            },
        ));

        //This is a hack because the WASM version is dropping this render pipeline and
        //   uniform group for some reason.
        engine.renderer.cam_uniform_group = self.cam_uniform_group.take();
        engine.renderer.render_3D_pipeline = self.render_pipeline.take();
        //engine.renderer.bind_group_layout = Some(bind_group_layout);
    }
}
