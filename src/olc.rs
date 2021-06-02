#![allow(non_snake_case)]
#![allow(deprecated)]
//#![feature(nll)]

mod app;
mod gl_const;
use crate::{camera, game_object, geometry, Point};
use core::{fmt, ops};
use ops::{Deref, DerefMut};
use std::ffi::{CStr, CString};
use std::fmt::Error;
use std::mem::size_of;
use std::ops::Try;
use std::sync::mpsc::channel;
use std::sync::{Arc, MutexGuard, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{mem, thread};

use lazy_static::*;
use pretty_hex::*;

use gl_const::*;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::geometry::{Triangle, UV};
use std::fs::File;

use png::Transformations;
use winit::window::{Window, WindowAttributes};

#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowBuilderExtWebSys;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

#[cfg(not(target_arch = "wasm32"))]
use winit::platform::windows::EventLoopExtWindows;

use futures::executor::block_on;
use futures::StreamExt;
use std::collections::HashMap;
use std::io::Write;
use std::num::NonZeroU32;
use wgpu::util::DeviceExt;
use wgpu::{Texture, TextureView};
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ElementState, MouseButton, MouseScrollDelta};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

#[cfg(target_arch = "wasm32")]
use web_sys::console::trace;

const MOUSE_BUTTONS: u8 = 5;
const DEFAULT_ALPHA: u8 = 0xFF;
const DEFAULT_PIXEL: u32 = 0xFF << 24;

/*lazy_static! {
    static ref GL: GLLoader = GLLoader::construct();
}*/
//static mut PGE: OnceCell<OLCEngine> = OnceCell::new();

pub enum Rcode {
    Fail,
    Ok,
    NoFile,
}

impl Try for Rcode {
    type Ok = Rcode;
    type Error = Rcode;

    fn into_result(self) -> Result<Rcode, Rcode> {
        match self {
            Rcode::Ok => Ok(Rcode::Ok),
            Rcode::Fail => Err(Rcode::Fail),
            Rcode::NoFile => Err(Rcode::NoFile),
        }
    }

    fn from_error(v: Rcode) -> Self {
        v
    }

    fn from_ok(v: Rcode) -> Self {
        v
    }
}

pub struct Renderer {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub sc_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub render_pipeline: Option<wgpu::RenderPipeline>,
    pub render_3D_pipeline: Option<wgpu::RenderPipeline>,
    pub decal_buffer: wgpu::Buffer,
    pub decals: Vec<Texture>,
    pub decal_views: Vec<TextureView>,
    pub active_decals: Vec<i32>,
    pub decal_counter: i32,
    pub decal_sampler: Option<wgpu::Sampler>,
    pub bind_group_layout: Option<wgpu::BindGroupLayout>,
    pub bind_group: Option<wgpu::BindGroup>,
    pub textures: Option<Vec<wgpu::BindGroup>>,
    pub frame: Option<wgpu::SwapChainFrame>,
    pub layer_shader: wgpu::ShaderModule,
    pub base_3d_shader: wgpu::ShaderModule,
    pub camera_buffer: wgpu::Buffer,
    pub cam_uniform_group: Option<wgpu::BindGroup>,
    pub meshes: Vec<geometry::Mesh>,
    pub points: Vec<Point>,
    pub vertex_buffer: wgpu::Buffer,
    pub point_buffer: wgpu::Buffer,
}

impl Renderer {
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let adapter_info = adapter.get_info();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::NON_FILL_POLYGON_MODE,
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap();
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: adapter.get_swap_chain_preferred_format(&surface).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);


        let layer_shader = device.create_shader_module(
            &wgpu::ShaderModuleDescriptor{
                label: Some("layer_shader"),
                flags: wgpu::ShaderFlags::all(),
                source: wgpu::ShaderSource::Wgsl(include_str!("layer.wgsl").into()),
            });

        let base_3d_shader = device.create_shader_module(
            &wgpu::ShaderModuleDescriptor{
                label: Some("base_3d_shader"),
                flags: wgpu::ShaderFlags::all(),
                source: wgpu::ShaderSource::Wgsl(include_str!("persp.wgsl").into()),
            });

        let decal_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Decal Buffer"),
            usage: wgpu::BufferUsage::VERTEX,
            #[cfg(not(target_arch = "wasm32"))]
            contents: bytemuck::cast_slice(&[
                geometry::Vertex {
                    position: [1.0, 1.0, 0.0],
                    tex_coords: [1.0, 0.0, 0.0],
                },
                geometry::Vertex {
                    position: [-1.0, 1.0, 0.0],
                    tex_coords: [0.0, 0.0, 0.0],
                },
                geometry::Vertex {
                    position: [1.0, -1.0, 0.0],
                    tex_coords: [1.0, 1.0, 0.0],
                },
                geometry::Vertex {
                    position: [-1.0, -1.0, 0.0],
                    tex_coords: [0.0, 1.0, 0.0],
                },
            ]),
            #[cfg(target_arch = "wasm32")]
            contents: bytemuck::cast_slice(&[
                geometry::Vertex {
                    position: [1.0, 1.0, 0.0],
                    tex_coords: [1.0, 0.0, 0.0],
                },
                geometry::Vertex {
                    position: [-1.0, 1.0, 0.0],
                    tex_coords: [0.0, 0.0, 0.0],
                },
                geometry::Vertex {
                    position: [1.0, -1.0, 0.0],
                    tex_coords: [1.0, 1.0, 0.0],
                },
                geometry::Vertex {
                    position: [-1.0, -1.0, 0.0],
                    tex_coords: [0.0, 1.0, 0.0],
                },
            ]),
        });

        /* let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sampler"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });*/
        let decal_sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        let decals = vec![];
        let decal_views = vec![];
        let active_decals = vec![];
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(geometry::Primitives::cube().vertices().as_slice()),
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        });
        let point_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&[Point::default(); 4096]),
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        let mut cam_data: Vec<u8> = bytemuck::cast_slice(&[camera::RawMat::default()]).into();
        let window_size = unsafe { PLATFORM_DATA.window_size.as_ref().unwrap() };
        unsafe {
            let x_bytes: [u8; 4] = std::mem::transmute(window_size.x);
            let y_bytes: [u8; 4] = std::mem::transmute(window_size.y);
            cam_data.extend_from_slice(&x_bytes);
            cam_data.extend_from_slice(&y_bytes);
        }
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: cam_data.as_slice(),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });
        let cam_uniform_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        }));
        Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            size,
            render_pipeline: None,
            render_3D_pipeline: None,
            decal_buffer,
            decals,
            decal_views,
            active_decals,
            decal_counter: 0,
            decal_sampler: Some(decal_sampler),
            bind_group_layout: None,
            bind_group: None,
            textures: None,
            frame: None,
            base_3d_shader,
            camera_buffer,
            cam_uniform_group,
            meshes: vec![],
            points: vec![],
            vertex_buffer,
            point_buffer,
            layer_shader,
        }
    }

    pub fn setup_layer_pipeline(&mut self) {
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("bind group layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStage::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStage::FRAGMENT,
                            ty: wgpu::BindingType::Sampler {
                                comparison: false,
                                filtering: false,
                            },
                            count: None,
                        },
                    ],
                });
        let render_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });
        let vert_descriptor = wgpu::VertexState {
            module: &self.layer_shader,
            entry_point: "vs_main",                  // 1.
            buffers: &[geometry::Vertex::desc()], // 2.
        };
        let sc_desc = &[wgpu::ColorTargetState {
            format: self.sc_desc.format,
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
        }];
        let pipe_line_desc = wgpu::RenderPipelineDescriptor {
            label: Some("Setup Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: vert_descriptor,
            fragment: Some(wgpu::FragmentState {
                // 3.
                module: &self.layer_shader,
                entry_point: "fs_main",
                targets: sc_desc,
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip, // 1.
                strip_index_format: Some(wgpu::IndexFormat::Uint32),
                front_face: wgpu::FrontFace::Ccw, // 2.
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                clamp_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None, // 1.
            multisample: wgpu::MultisampleState::default(),
        };
        self.bind_group_layout = Some(bind_group_layout);
        self.render_pipeline = Some(self.device.create_render_pipeline(&pipe_line_desc));
    }

    pub fn setup_3D_pipeline(&mut self) {
        let bind_group_layout =
            self.device
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
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });
        self.render_3D_pipeline = Some(self.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &self.base_3d_shader,
                    entry_point: "vs_main",
                    buffers: &[super::Point::desc()],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw, // 2.
                    cull_mode: Some(wgpu::Face::Back),
                    // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                    clamp_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &self.base_3d_shader,
                    entry_point: "fs_main",
                    targets: &[wgpu::ColorTargetState {
                        format: self.sc_desc.format,
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
    }

    fn update_texture_groups(&mut self) {
        let mut active_tex: Vec<&TextureView> = self
            .active_decals
            .iter()
            .map(|k| &self.decal_views[*k as usize])
            .collect();
        let mut tex_group: Vec<wgpu::BindGroup> = vec![];
        for tex in active_tex {
            tex_group.insert(
                tex_group.len(),
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(tex),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(
                                self.decal_sampler.as_ref().unwrap(),
                            ),
                        },
                    ],
                    layout: &self
                        .bind_group_layout
                        .as_ref()
                        .expect("No Bind Group Layout"),
                    label: Some("bind group"),
                }),
            );
        }
        self.textures = Some(tex_group);
    }

    pub fn update_viewport(&mut self, position: Vi2d, size: Vi2d) -> Rcode {
        self.size = winit::dpi::PhysicalSize {
            width: size.x as u32,
            height: size.y as u32,
        };
        self.sc_desc.width = size.x as u32;
        self.sc_desc.height = size.y as u32;
        //self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
        Rcode::Ok
    }

    pub fn new_frame(&mut self) {
        match self.frame.as_ref() {
            None => {
                self.frame = Some(match self.swap_chain.get_current_frame() {
                    Ok(frame) => frame,
                    Err(_) => {
                        self.swap_chain =
                            self.device.create_swap_chain(&self.surface, &self.sc_desc);
                        self.swap_chain
                            .get_current_frame()
                            .expect("Failed to acquire next swap chain texture!")
                    }
                });
            }
            Some(_) => {}
        }
    }

    pub fn get_frame(&self) -> Result<&wgpu::SwapChainFrame, ()> {
        Ok(self.frame.as_ref().expect("Failed to get frame"))
    }

    pub fn clear_frame(&mut self) {
        self.frame = None;
    }

    pub fn add_mesh(&mut self, mesh: geometry::Mesh) {
        self.meshes.insert(self.meshes.len(), mesh);
        let mut verts: Vec<geometry::Vertex> = vec![];
        for mesh in self.meshes.iter_mut() {
            verts.append(&mut mesh.vertices());
        }
        self.vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(verts.as_slice()),
                usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            });
    }

    pub fn add_game_object(&mut self, go: game_object::GameObject) {
        if let Some(mesh) = go.mesh {
            self.add_mesh(mesh);
        }
    }

    pub fn draw_3D(&mut self, camera: &camera::Camera, encoder: &mut wgpu::CommandEncoder) {
        if self.meshes.len() < 1 {
            return;
        }

        let mut verts: Vec<geometry::Vertex> = vec![];
        for mesh in self.meshes.iter_mut() {
            verts.append(&mut mesh.vertices());
        }
        let frame = self.get_frame().expect("Couldn't get frame");
        let mut cam_data: Vec<u8> = bytemuck::cast_slice(&[camera.mat]).into();
        let window_size = unsafe { PLATFORM_DATA.window_size.as_ref().unwrap() };
        let (x, y) = (window_size.x as f32, window_size.y as f32);
        let x_bytes: [u8; 4] = x.to_ne_bytes();
        let y_bytes: [u8; 4] = y.to_ne_bytes();
        cam_data.extend_from_slice(&x_bytes);
        cam_data.extend_from_slice(&y_bytes);
        /*(std::mem::size_of::<Triangle>() *
        geometry::Primitives::cube().tris.len()) as u64*/
        self.queue
            .write_buffer(&self.camera_buffer, 0, cam_data.as_slice());
        //self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(verts.as_slice()));
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &frame.output.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            if let Some(render_pipeline) = self.render_3D_pipeline.as_ref() {
                render_pass.set_pipeline(render_pipeline);
            } else {
                panic!("NO RENDER PIPELINE")
            }

            render_pass.set_bind_group(0, self.cam_uniform_group.as_ref().unwrap(), &[]);

            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            let vert_count = self
                .meshes
                .iter()
                .fold(0, |acc, m| acc + (m.vertices().len() as u32));
            render_pass.draw(0..vert_count, 0..1);
        }
    }
    pub fn draw_points(&mut self, camera: &camera::Camera, encoder: &mut wgpu::CommandEncoder) {
        let frame = self.get_frame().expect("Couldn't get frame");
        let mut cam_data: Vec<u8> = bytemuck::cast_slice(&[camera.mat]).into();
        let window_size = unsafe { PLATFORM_DATA.window_size.as_ref().unwrap() };
        let (x, y) = (window_size.x as f32, window_size.y as f32);
        let x_bytes: [u8; 4] = x.to_ne_bytes();
        let y_bytes: [u8; 4] = y.to_ne_bytes();
        cam_data.extend_from_slice(&x_bytes);
        cam_data.extend_from_slice(&y_bytes);
        /*(std::mem::size_of::<Triangle>() *
        geometry::Primitives::cube().tris.len()) as u64*/
        self.queue
            .write_buffer(&self.camera_buffer, 0, cam_data.as_slice());
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &frame.output.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            if let Some(render_pipeline) = self.render_3D_pipeline.as_ref() {
                render_pass.set_pipeline(render_pipeline);
            } else {
                panic!("NO RENDER PIPELINE")
            }

            render_pass.set_bind_group(0, self.cam_uniform_group.as_ref().unwrap(), &[]);

            render_pass.set_vertex_buffer(0, self.point_buffer.slice(..));

            for i in 0..1 {
                self.queue.write_buffer(
                    &self.point_buffer,
                    0,
                    bytemuck::cast_slice(self.points.as_slice()),
                );
                render_pass.draw(0..self.points.len() as u32, 0..1);
            }
        }
    }
    pub fn draw_layers(&mut self, encoder: &mut wgpu::CommandEncoder) -> Rcode {
        let frame = self.get_frame().expect("Couldn't get frame");

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &frame.output.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            if let Some(render_pipeline) = self.render_pipeline.as_ref() {
                render_pass.set_pipeline(render_pipeline);
            }
            if let Some(textures) = self.textures.as_ref() {
                for tex_group in textures {
                    render_pass.set_bind_group(0, tex_group, &[]);
                    render_pass.set_vertex_buffer(0, self.decal_buffer.slice(..));
                    render_pass.draw(0..4, 0..1);
                }
            }
        }
        Rcode::Ok
    }

    pub fn clear_buffer(&mut self, p: Pixel, depth: bool) {
        let frame = self
            .swap_chain
            .get_current_frame()
            .expect("Failed to Get Frame")
            .output;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: p.r() as f64 / 255.0,
                            g: p.g() as f64 / 255.0,
                            b: p.b() as f64 / 255.0,
                            a: p.a() as f64 / 255.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
        }
        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    pub fn create_texture(&mut self, width: u32, height: u32) -> i32 {
        let texture_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsage::SAMPLED
                | wgpu::TextureUsage::COPY_DST
                | wgpu::TextureUsage::RENDER_ATTACHMENT
                | wgpu::TextureUsage::COPY_SRC,
            label: None,
        });
        let tex_view = tex.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        self.decals.insert(self.decal_counter as usize, tex);
        self.decal_views
            .insert(self.decal_counter as usize, tex_view);
        self.decal_counter += 1;
        self.decal_counter - 1
    }

    pub fn delete_texture(mut id: &mut u32) -> u32 {
        0
    }

    pub fn apply_texture(id: u32) {
        //add Layer View TextureViews in renderer.texture_views
    }

    pub fn update_texture(&self, _id: u32, spr: &Sprite) {
        let size = wgpu::Extent3d {
            width: spr.width,
            height: spr.height,
            depth_or_array_layers: 1,
        };
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                texture: &self.decals[_id as usize],
            },
            spr.get_data(),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(4 * spr.width as u32),
                rows_per_image: NonZeroU32::new(0),
            },
            size,
        );
    }
    pub fn update_texture_region(
        _id: u32,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        data: &[Pixel],
    ) {
    }
    pub fn draw_layer_quad(offset: Vf2d, scale: Vf2d, tint: Pixel) {
        //The () functions are because accessing the union is unsafe, and I
        // don't like leaving unsafe{} all over the place.
    }

    pub fn draw_decal_quad(decal: &mut DecalInstance) {
        //I'm wrapping this whole thing in unsafe because
        // it accesses Union values
        /*unsafe {
            if decal.decal.is_none() {
                (GL.glBindTexture)(GL_TEXTURE_2D, 0);

                (GL.glBegin)(GL_QUADS);
                (GL.glColor4ub)(decal.tint[0].rgba.0, decal.tint[0].rgba.1,
                                decal.tint[0].rgba.2, decal.tint[0].rgba.3);
                (GL.glTexCoord4f)(decal.uv[0].x, decal.uv[0].y, 0.0, decal.w[0]);
                (GL.glVertex2f)(decal.pos[0].x, decal.pos[0].y);

                (GL.glColor4ub)(decal.tint[1].rgba.0, decal.tint[1].rgba.1,
                                decal.tint[1].rgba.2, decal.tint[1].rgba.3);
                (GL.glTexCoord4f)(decal.uv[1].x, decal.uv[1].y, 0.0, decal.w[1]);
                (GL.glVertex2f)(decal.pos[1].x, decal.pos[1].y);

                (GL.glColor4ub)(decal.tint[2].rgba.0, decal.tint[2].rgba.1,
                                decal.tint[2].rgba.2, decal.tint[2].rgba.3);
                (GL.glTexCoord4f)(decal.uv[2].x, decal.uv[2].y, 0.0, decal.w[2]);
                (GL.glVertex2f)(decal.pos[2].x, decal.pos[2].y);

                (GL.glColor4ub)(decal.tint[3].rgba.0, decal.tint[3].rgba.1,
                                decal.tint[3].rgba.2, decal.tint[3].rgba.3);
                (GL.glTexCoord4f)(decal.uv[3].x, decal.uv[3].y, 0.0, decal.w[3]);
                (GL.glVertex2f)(decal.pos[3].x, decal.pos[3].y);
                (GL.glEnd)();
            } else {
                let decal_id = decal.get().id;
                (GL.glBindTexture)(GL_TEXTURE_2D, decal_id as u32);
                (GL.glBegin)(GL_QUADS);
                (GL.glColor4ub)(decal.tint[0].rgba.0, decal.tint[0].rgba.1,
                                decal.tint[0].rgba.2, decal.tint[0].rgba.3);

                (GL.glTexCoord4f)(decal.uv[0].x, decal.uv[0].y, 0.0, decal.w[0]);
                (GL.glVertex2f)(decal.pos[0].x, decal.pos[0].y);

                (GL.glTexCoord4f)(decal.uv[1].x, decal.uv[1].y, 0.0, decal.w[1]);
                (GL.glVertex2f)(decal.pos[1].x, decal.pos[1].y);

                (GL.glTexCoord4f)(decal.uv[2].x, decal.uv[2].y, 0.0, decal.w[2]);
                (GL.glVertex2f)(decal.pos[2].x, decal.pos[2].y);

                (GL.glTexCoord4f)(decal.uv[3].x, decal.uv[3].y, 0.0, decal.w[3]);
                (GL.glVertex2f)(decal.pos[3].x, decal.pos[3].y);
                (GL.glEnd)();
            }
        }*/
    }
    pub fn draw_triangles(triangles: &[Triangle], texture: u32) {}
}

pub trait Platform {
    fn create_window_pane(
        window_pos: Vi2d,
        window_size: Vi2d,
        full_screen: bool,
    ) -> (Window, EventLoop<()>);
    fn application_startup(&self) -> Rcode {
        Rcode::Ok
    }
    fn application_cleanup(&self) -> Rcode {
        Rcode::Ok
    }
    fn thread_startup(&self) -> Rcode {
        Rcode::Ok
    }
    fn thread_cleanup(&self) -> Rcode {
        Rcode::Ok
    }
    fn create_graphics(
        &mut self,
        full_screen: bool,
        enable_vsync: bool,
        view_pos: Vi2d,
        view_size: Vi2d,
    ) -> Rcode {
        Rcode::Ok
    }
    fn set_window_title(window: &Window, title: String) -> Rcode {
        Rcode::Ok
    }
    fn handle_window_event(window: &Window, event: &Event<()>);
    fn handle_system_event_loop(&self) -> Rcode {
        Rcode::Ok
    }
    fn handle_system_event(&self) -> Rcode {
        Rcode::Ok
    }
}

//#[cfg(not(target_arch = "wasm32"))]
pub struct PlatformWindows {
    window: Window,
    event_loop: EventLoop<()>,
}
/*
#[cfg(target_arch = "wasm32")]
pub struct PlatformWeb {
    window: Window,
    event_loop: EventLoop<()>,
}
*/

static mut PLATFORM_DATA: PlatformData = PlatformData::create();

//this is only ever updated from the Platform thread,
// so immutable references to it are thread safe
pub type Key = winit::event::VirtualKeyCode;
struct PlatformData {
    mouse_focus: bool,
    key_focus: bool,
    new_key_state_map: Option<HashMap<Key, bool>>,
    old_key_state_map: Option<HashMap<Key, bool>>,
    new_mouse_state_map: Option<Vec<bool>>,
    old_mouse_state_map: Option<Vec<bool>>,
    key_map: Option<HashMap<Key, HWButton>>,
    mouse_map: Option<Vec<HWButton>>,
    mouse_wheel_delta: i32,
    mouse_wheel_delta_cache: i32,
    mouse_position: Option<Vf2d>,
    raw_mouse_position: Option<Vf2d>,
    view_position: Option<Vi2d>,
    window_size: Option<Vi2d>,
    resolution: Option<Vi2d>,
    screen_size: Option<Vi2d>,
    pixel_size: Option<Vi2d>,
    mouse_position_cache: Option<Vf2d>,
    window_alive: bool,
    full_screen: bool,
    vsync: bool,
    title: String,
    running: bool,
}

impl PlatformData {
    const fn create() -> Self {
        Self {
            mouse_focus: false,
            key_focus: false,
            new_key_state_map: None,
            old_key_state_map: None,
            new_mouse_state_map: None,
            old_mouse_state_map: None,
            key_map: None,
            mouse_map: None,
            mouse_wheel_delta: 0,
            mouse_wheel_delta_cache: 0,
            mouse_position: None,
            raw_mouse_position: None,
            view_position: None,
            window_size: None,
            pixel_size: None,
            resolution: None,
            screen_size: None,
            mouse_position_cache: None,
            window_alive: true,
            full_screen: false,
            vsync: false,
            title: String::new(),
            running: true,
        }
    }
    fn init(&mut self) {
        self.new_key_state_map = Some(HashMap::default());
        self.old_key_state_map = Some(HashMap::default());
        self.new_mouse_state_map = Some(vec![false; 3]);
        self.old_mouse_state_map = Some(vec![false; 3]);
        self.key_map = Some(HashMap::default());
        self.mouse_map = Some(vec![HWButton::new(); 3]);
        self.mouse_position = Some((0.0, 0.0).into());
        self.view_position = Some((0, 0).into());
        self.window_size = Some((0, 0).into());
        self.resolution = Some((0, 0).into());
        self.screen_size = Some((0, 0).into());
        self.pixel_size = Some((0, 0).into());
        self.mouse_position_cache = Some((0.0, 0.0).into());
    }

    fn update_mouse(&mut self, mut x: i32, mut y: i32) {
        self.raw_mouse_position = Some(
            (
                x as f32 + self.view_position.unwrap_or_default().x as f32,
                y as f32 + self.view_position.unwrap_or_default().y as f32,
            )
                .into(),
        );
        self.mouse_focus = true;
        let px_i = self.pixel_size.unwrap_or_default();
        let px: Vf2d = (px_i.x as f32, px_i.y as f32).into();
        let mut temp_mouse: Vi2d = ((x as f32 / px.x) as i32, (y as f32 / px.y) as i32).into();
        if temp_mouse.x >= (self.window_size.unwrap_or_default().x as f32 / px.x).floor() as i32 {
            temp_mouse.x = (self.window_size.unwrap_or_default().x as f32 / px.x) as i32 - 1
        }
        if temp_mouse.y >= (self.window_size.unwrap_or_default().y as f32 / px.y).floor() as i32 {
            temp_mouse.y = (self.window_size.unwrap_or_default().y as f32 / px.y) as i32 - 1
        }
        //log::trace!("{:x?}", temp_mouse);
        if temp_mouse.x < 0 {
            temp_mouse.x = 0
        }
        if temp_mouse.y < 0 {
            temp_mouse.y = 0
        }
        self.mouse_position_cache = Some((temp_mouse.x as f32, temp_mouse.y as f32).into());
    }
    fn update_window_size(&mut self, width: u32, height: u32) {
        self.window_size = Some(Vi2d::from(((width as i32), (height as i32))));
    }
    fn update_window_position(&mut self, x: i32, y: i32) {
        self.view_position = Some(Vi2d::from(((x as i32), (y as i32))));
    }
    fn update_mouse_wheel(&mut self, delta: i32) {
        self.mouse_wheel_delta_cache += delta;
    }
    fn update_mouse_focus(&mut self, b: bool) {
        self.mouse_focus = b
    }
    fn update_key_focus(&mut self, b: bool) {
        self.key_focus = b
    }
    fn update_key_state(&mut self, k: Key, b: bool) {
        *self
            .new_key_state_map
            .as_mut()
            .unwrap()
            .entry(k)
            .or_insert(b) = b;
    }
    fn update_mouse_state(&mut self, i: i32, b: bool) {
        self.new_mouse_state_map.as_mut().unwrap()[i as usize] = b;
    }
}

//#[cfg(not(target_arch = "wasm32"))]
impl PlatformWindows {
    pub fn new() -> PlatformWindows {
        let event_loop = EventLoop::new();
        PlatformWindows {
            window: WindowBuilder::new().build(&event_loop).unwrap(),
            event_loop,
        }
    }
}

//#[cfg(not(target_arch = "wasm32"))]
impl Platform for PlatformWindows {
    fn create_window_pane(
        window_pos: Vi2d,
        window_size: Vi2d,
        full_screen: bool,
    ) -> (Window, EventLoop<()>) {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_inner_size(winit::dpi::Size::Logical(LogicalSize {
                width: window_size.x as f64,
                height: window_size.y as f64,
            }))
            /*.with_fullscreen(
            Some(winit::window::Fullscreen::Borderless(
            event_loop.available_monitors().next().expect("Wrong monitor"))))*/
            .build(&event_loop)
            .expect("Failed to build Window");

        (window, event_loop)
    }

    fn set_window_title(window: &Window, title: String) -> Rcode {
        window.set_title(&title);
        unsafe { PLATFORM_DATA.title = title }
        Rcode::Ok
    }

    fn handle_window_event(window: &Window, event: &Event<()>) {
        unsafe {
            if let Event::WindowEvent {
                window_id: _,
                ref event,
            } = event
            {
                match event {
                    WindowEvent::CursorMoved {
                        device_id: _,
                        position,
                        modifiers: _,
                    } => {
                        PLATFORM_DATA.update_mouse(position.x as i32, position.y as i32);
                    }
                    WindowEvent::Resized(size) => {
                        PLATFORM_DATA.update_window_size(size.width, size.height);
                    }
                    WindowEvent::Moved(position) => {
                        PLATFORM_DATA.update_window_position(position.x, position.y);
                    }
                    WindowEvent::MouseWheel {
                        device_id: _,
                        delta: MouseScrollDelta::LineDelta(h, v),
                        phase,
                        modifiers: _,
                    } => {
                        PLATFORM_DATA.update_mouse_wheel(*v as i32);
                    }
                    WindowEvent::CursorLeft { device_id: _ } => {
                        PLATFORM_DATA.update_mouse_focus(false);
                    }
                    WindowEvent::Focused(focus) => {
                        PLATFORM_DATA.update_key_focus(*focus);
                    }
                    WindowEvent::KeyboardInput {
                        device_id: _,
                        input,
                        is_synthetic,
                    } => {
                        if let Some(key) = input.virtual_keycode {
                            PLATFORM_DATA
                                .update_key_state(key, input.state == ElementState::Pressed);
                        }
                    }
                    WindowEvent::MouseInput {
                        device_id: _,
                        state,
                        button,
                        modifiers: _,
                    } => match button {
                        winit::event::MouseButton::Left => {
                            PLATFORM_DATA.update_mouse_state(0, state == &ElementState::Pressed)
                        }
                        winit::event::MouseButton::Right => {
                            PLATFORM_DATA.update_mouse_state(1, state == &ElementState::Pressed)
                        }
                        winit::event::MouseButton::Middle => {
                            PLATFORM_DATA.update_mouse_state(2, state == &ElementState::Pressed)
                        }
                        winit::event::MouseButton::Other(b) => PLATFORM_DATA
                            .update_mouse_state(*b as i32, state == &ElementState::Pressed),
                    },
                    _ => {}
                }
            }
        }
    }

    fn handle_system_event(&self) -> Rcode {
        Rcode::Fail
    }
}

pub struct OLCEngine {
    pub app_name: String,
    pub is_focused: bool,
    pub window_width: u32,
    pub window_height: u32,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub pixels_w: u32,
    pub pixels_h: u32,
    pub fps: u32,
    pub renderer: Renderer,
    pub camera: camera::Camera,
    inv_screen_size: Vf2d,
    draw_target: u32,
    full_screen: bool,
    vsync: bool,
    layers: Vec<LayerDesc>,
    mouse_position: Vi2d,
    font_decal: Decal,
    depth_buffer: Vec<f64>,
    pub window: winit::window::Window,
}

impl OLCEngine {
    pub fn init(
        &mut self,
        app_name: &str,
        screen_width: u32,
        screen_height: u32,
        pixel_width: u32,
        pixel_height: u32,
        full_screen: bool,
        vsync: bool,
    ) {
        let tex_size_w = screen_width / pixel_width;
        let tex_size_h = screen_height / pixel_height;
        let inv_screen_size = Vf2d::from(((1.0 / tex_size_w as f32), (1.0 / tex_size_h as f32)));

        self.app_name = String::from(app_name);
        self.is_focused = true;
        self.window_width = screen_width;
        self.window_height = screen_height;
        self.pixels_w = tex_size_w;
        self.pixels_h = tex_size_h;
        self.pixel_width = pixel_width;
        self.pixel_height = pixel_height;
        self.inv_screen_size = inv_screen_size;
        self.fps = 0;
        self.full_screen = full_screen;
        self.vsync = vsync;
        self.layers = vec![];
        self.draw_target = 0;
        self.mouse_position = Vi2d::new(0, 0);
        self.font_decal = Decal::empty();
    }

    pub fn is_focused(&self) -> bool {
        unsafe { PLATFORM_DATA.key_focus }
    }

    pub fn get_key(&self, k: Key) -> HWButton {
        unsafe {
            if let Some(button) = PLATFORM_DATA.key_map.as_mut().unwrap().get(&k) {
                *button
            } else {
                HWButton::new()
            }
        }
    }

    fn set_key(&self, k: Key, hw: HWButton) {
        unsafe {
            let key = PLATFORM_DATA
                .key_map
                .as_mut()
                .unwrap()
                .entry(k)
                .or_insert(hw);
            *key = hw;
        }
    }
    fn clear_keys(&self) {
        unsafe {
            for (key, value) in PLATFORM_DATA.key_map.as_mut().unwrap() {
                (*value).pressed = false;
                (*value).released = false;
            }
        }
    }

    pub fn get_mouse(&self, b: Mouse) -> HWButton {
        unsafe {
            match b {
                Mouse::LEFT => PLATFORM_DATA.mouse_map.as_ref().unwrap()[0],
                Mouse::RIGHT => PLATFORM_DATA.mouse_map.as_ref().unwrap()[1],
                Mouse::MIDDLE => PLATFORM_DATA.mouse_map.as_ref().unwrap()[2],
            }
        }
    }

    pub fn render_gl_tris(&mut self, triangles: &[Triangle], texture: u32) {
        Renderer::draw_triangles(triangles, texture);
    }

    pub fn set_perspective(&self, fov: f32, aspect: f32, near_clip: f32, far_clip: f32) {
        let f_H = ((fov / 360.0 * std::f32::consts::PI) * near_clip) as f64;
        let f_W = f_H * aspect as f64;
        //(GL.glFrustum)(-f_W, f_W, -f_H, f_H, near_clip as f64, far_clip as f64)
    }

    pub fn set_mouse(&self, i: usize, k: HWButton) {
        unsafe { PLATFORM_DATA.mouse_map.as_mut().unwrap()[i] = k }
    }

    pub fn set_mouse_pos(&self, x: f32, y: f32) {
        unsafe {
            let px = PLATFORM_DATA.pixel_size.unwrap_or_default();
            let mp: Vi2d = (
                x.floor() as i32 * px.x + PLATFORM_DATA.view_position.unwrap().x,
                y.floor() as i32 * px.y + PLATFORM_DATA.view_position.unwrap().y,
            )
                .into();
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.window
                    .set_cursor_position(winit::dpi::PhysicalPosition { x: mp.x, y: mp.y });
            }

            #[cfg(target_arch = "wasm32")]
            {}

            /*if winapi::um::winuser::SetCursorPos(mp.x, mp.y) == 0
            {
                panic!("SetCursorPos failed");
            }*/
            PLATFORM_DATA.raw_mouse_position = Some((mp.x as f32, mp.y as f32).into());
            PLATFORM_DATA.mouse_position = Some((x.floor(), y.floor()).into());
        }
    }

    #[cfg(feature = "web-sys")]
    fn set_web_mouse(&self) {}

    #[cfg(feature = "web-sys")]
    fn check_mouse_lock(&self) -> bool {
        //self.window.has_pointer_grab()
        false
    }

    #[cfg(target_arch = "wasm32")]
    fn request_mouse_lock(&self) {
        self.window
            .set_cursor_grab(true)
            .expect("Can't grab cursor");
    }

    pub fn clear_depth_buffer(&mut self) {
        self.depth_buffer = vec![0.0; (self.pixels_w * self.pixels_h) as usize];
    }

    pub fn update_depth_buffer(&mut self, i: usize, d: f64) {
        self.depth_buffer[i] = d;
    }

    pub fn check_depth_buffer(&mut self, i: usize) -> f64 {
        self.depth_buffer[i]
    }

    pub fn mouse_x(&self) -> f32 {
        unsafe { PLATFORM_DATA.mouse_position.unwrap_or_default().x }
    }

    pub fn mouse_y(&self) -> f32 {
        unsafe { PLATFORM_DATA.mouse_position.unwrap_or_default().y }
    }

    pub fn mouse_wheel(&self) -> i32 {
        0
    }

    //pub fn get_window_mouse() -> Vi2d { Vi2d }

    pub fn get_mouse_pos(&self) -> Vf2d {
        unsafe {
            let m = PLATFORM_DATA.mouse_position;
            m.unwrap()
        }
    }
    pub fn get_raw_mouse_pos(&self) -> Vf2d {
        unsafe {
            let m = PLATFORM_DATA.raw_mouse_position;
            m.unwrap()
        }
    }

    //Utility
    pub fn screen_width(&self) -> i32 {
        unsafe { PLATFORM_DATA.screen_size.unwrap_or_default().x }
    }

    pub fn screen_height(&self) -> i32 {
        unsafe { PLATFORM_DATA.screen_size.unwrap_or_default().y }
    }

    pub fn get_draw_target_width(&self) -> i32 {
        self.get_draw_target_ref().sprite.width as i32
    }

    pub fn get_draw_target_height(&self) -> i32 {
        self.get_draw_target_ref().sprite.height as i32
    }

    pub fn set_screen_size(&self, w: i32, h: i32) {}

    //This is overriden by drawing onto Layers.
    //We may want to do a "DrawTarget" Trait.
    //pub fn set_draw_target(target: &mut Sprite) {}
    //pub fn get_draw_target() -> Sprite { Sprite }

    pub fn get_fps(&self) -> i32 {
        0
    }

    pub fn get_elapsed_time(&self) -> f32 {
        0.0
    }

    pub fn get_window_size(&self) -> Vi2d {
        unsafe { PLATFORM_DATA.window_size.unwrap() }
    }
    pub fn get_window_position(&self) -> Vi2d {
        unsafe { PLATFORM_DATA.view_position.unwrap() }
    }

    pub fn get_screen_size_in_game_pixels(&self) -> Vi2d {
        Vi2d::new(self.window_width as i32, self.window_height as i32)
    }
    pub fn get_pixel_size(&self) -> Vi2d {
        Vi2d::new(self.pixel_width as i32, self.pixel_height as i32)
    }

    pub fn set_draw_target(&mut self, layer_id: u32) {
        self.draw_target = layer_id;
        self.set_layer_update(layer_id, true);
    }
    pub fn reset_draw_target(&mut self) {
        //set back to background layer
        self.draw_target = self.layers[0].id;
    }

    pub fn get_draw_target(&mut self) -> Result<&mut LayerDesc, ()> {
        let layer_iter = self.layers.iter_mut();
        for layer in layer_iter {
            if layer.id == self.draw_target {
                return Ok(layer);
            }
        }
        Err(())
    }

    pub fn get_draw_target_ref(&self) -> &LayerDesc {
        self.get_layer_ref(self.draw_target).unwrap()
    }

    pub fn set_layer_visible(&mut self, layer_id: u32, b: bool) {
        let layer_iter = self.layers.iter_mut();
        for layer in layer_iter {
            if layer.id == layer_id {
                layer.shown = b;
            }
        }
    }

    pub fn set_layer_update(&mut self, layer_id: u32, b: bool) {
        let layer_iter = self.layers.iter_mut();
        for layer in layer_iter {
            if layer.id == layer_id {
                layer.update = b;
            }
        }
    }

    pub fn set_layer_offset(&self, layer: u8, x: f32, y: f32) {}

    pub fn set_layer_scale(&self, layer: u8, x: f32, y: f32) {}

    pub fn set_layer_tint(&self, layer: u8, tint: Pixel) {}

    //We'll come back to this
    //pub fn set_layer_custom_render_function

    pub fn get_layer_ref(&self, layer_id: u32) -> Option<&LayerDesc> {
        let layer_iter = self.layers.iter();
        for layer in layer_iter {
            if layer.id == layer_id {
                return Some(layer);
            }
        }
        None
    }

    pub fn get_layer(&self, layer_id: u32) -> Result<&LayerDesc, ()> {
        let layer_iter = self.layers.iter();
        for layer in layer_iter {
            if layer.id == layer_id {
                return Ok(layer);
            }
        }
        Err(())
    }
    pub fn get_layer_mut(&mut self, layer_id: u32) -> Result<&LayerDesc, ()> {
        let layer_iter = self.layers.iter();
        for layer in layer_iter {
            if layer.id == layer_id {
                return Ok(layer);
            }
        }
        Err(())
    }

    pub fn add_layer(&mut self) -> u32 {
        let lay_id = self.renderer.create_texture(self.pixels_w, self.pixels_h);
        let mut layer = LayerDesc::new(self.pixels_w, self.pixels_h);
        layer.id = lay_id as u32;
        self.layers.push(layer);
        self.renderer.active_decals.insert(lay_id as usize, lay_id);
        lay_id as u32
    }

    pub fn load_sprite(&self, path: &str) -> Sprite {
        Sprite::load_from_file::<BMPLoader>(path).unwrap()
    }

    fn push_decal_instance(&mut self, di: DecalInstance) {
        self.get_draw_target()
            .expect("Can't get draw target")
            .vec_decal_instance
            .push(di);
    }

    pub fn set_pixel_mode(&mut self, m: PixelMode) {}

    pub fn get_pixel_mode(&self) -> PixelMode {
        PixelMode::Normal
    }

    pub fn set_pixel_blend(&mut self, blend: f32) {}

    //DRAW ROUTINES
    pub fn draw(&mut self, x: i32, y: i32, p: Pixel) {
        self.get_draw_target()
            .expect("Can't get draw target")
            .sprite
            .set_pixel(x as u32, y as u32, p);
    }
    //DRAW ROUTINES
    pub fn draw_subregion(&mut self, x: i32, y: i32, width: i32, height: i32, p: &[Pixel]) {
        self.get_draw_target()
            .expect("Can't get draw target")
            .sprite
            .set_region(x as u32, y as u32, width as u32, height as u32, p);
    }

    pub fn draw_line(&mut self, pos1: Vi2d, pos2: Vi2d, p: Pixel, pattern: u32) {
        self.draw_line_xy(pos1.x, pos1.y, pos2.x, pos2.y, p);
    }

    pub fn draw_line_xy(&mut self, mut x1: i32, mut y1: i32, mut x2: i32, mut y2: i32, p: Pixel) {
        if x1 < 0
            || x2 < 0
            || x1 > self.pixels_w as i32
            || x2 > self.pixels_w as i32
            || y1 < 0
            || y2 < 0
            || y1 > self.pixels_h as i32
            || y2 > self.pixels_h as i32
        {
            return;
        }

        let (mut x, mut y, mut dx, mut dy, mut dx1, mut dy1, mut px, mut py, mut xe, mut ye, mut i) =
            (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
        dx = x2 - x1;
        dy = y2 - y1;

        if dx == 0 {
            if y2 < y1 {
                std::mem::swap(&mut y1, &mut y2);
            }
            for y in y1..=y2 {
                self.draw(x1, y, p);
            }
            return;
        }

        if dy == 0 {
            if x2 < x1 {
                std::mem::swap(&mut x1, &mut x2);
            }
            for x in x1..=x2 {
                self.draw(x, y1, p);
            }
            return;
        }

        dx1 = dx.abs();
        dy1 = dy.abs();
        px = 2 * dy1 - dx1;
        py = 2 * dx1 - dy1;
        if dy1 <= dx1 {
            if dx >= 0 {
                x = x1;
                y = y1;
                xe = x2;
            } else {
                x = x2;
                y = y2;
                xe = x1;
            }

            self.draw(x, y, p);

            for i in x..xe {
                x = x + 1;
                if px < 0 {
                    px = px + 2 * dy1;
                } else {
                    if (dx < 0 && dy < 0) || (dx > 0 && dy > 0) {
                        y += 1;
                    } else {
                        y = y - 1;
                    }
                    px = px + 2 * (dy1 - dx1);
                }
                self.draw(x, y, p);
            }
        } else {
            if dy >= 0 {
                x = x1;
                y = y1;
                ye = y2;
            } else {
                x = x2;
                y = y2;
                ye = y1;
            }
            self.draw(x, y, p);

            for i in y..ye {
                y = y + 1;
                if py <= 0 {
                    py = py + 2 * dx1;
                } else {
                    if (dx < 0 && dy < 0) || (dx > 0 && dy > 0) {
                        x = x + 1
                    } else {
                        x = x - 1;
                    }
                    py = py + 2 * (dx1 - dy1);
                }
                self.draw(x, y, p);
            }
        }
    }

    pub fn draw_circle(&mut self, pos: Vi2d, r: i32, p: Pixel, mask: u32) {
        self.draw_circle_xy(pos.x, pos.y, r, p, mask);
    }

    pub fn draw_circle_xy(&mut self, x: i32, y: i32, r: i32, p: Pixel, mask: u32) {
        if r < 0
            || x < -r
            || y < -r
            || x - self.get_draw_target_ref().sprite.width as i32 > r
            || y - self.get_draw_target_ref().sprite.height as i32 > r
        {
            return;
        }
        if r > 0 {
            let (mut x0, mut y0, mut d) = (0, r, 3 - 2 * r);
            while y0 >= x0 {
                if mask & 0x01 == 0x01 {
                    self.draw(x + x0, y - y0, p)
                };
                if mask & 0x04 == 0x04 {
                    self.draw(x + y0, y + x0, p)
                };
                if mask & 0x10 == 0x10 {
                    self.draw(x - x0, y + y0, p)
                };
                if mask & 0x40 == 0x40 {
                    self.draw(x - y0, y - x0, p)
                };
                if x0 != 0 && x0 != y0 {
                    if mask & 0x02 == 0x02 {
                        self.draw(x + y0, y - x0, p)
                    };
                    if mask & 0x08 == 0x08 {
                        self.draw(x + x0, y + y0, p)
                    };
                    if mask & 0x20 == 0x20 {
                        self.draw(x - y0, y + x0, p)
                    };
                    if mask & 0x80 == 0x80 {
                        self.draw(x - x0, y - y0, p)
                    };
                }
                x0 += 1;
                if d < 0 {
                    d += 4 * x0 + 6;
                } else {
                    y0 -= 1;
                    d += 4 * (x0 - y0) + 10;
                }
            }
        } else {
            self.draw(x, y, p);
        }
    }

    pub fn fill_circle(&mut self, pos: Vf2d, r: i32, p: Pixel) {
        self.fill_circle_xy(pos.x as i32, pos.y as i32, r, p);
    }

    pub fn fill_circle_xy(&mut self, mut x: i32, mut y: i32, r: i32, p: Pixel) {
        if r < 0
            || x < -r
            || y < -r
            || x - self.get_draw_target_ref().sprite.width as i32 > r
            || y - self.get_draw_target_ref().sprite.height as i32 > r
        {
            return;
        }

        if r > 0 {
            let (mut x0, mut y0, mut d) = (0, r, 3 - 2 * r);
            let mut drawline = |sx: i32, ex: i32, y: i32| {
                for x in sx..=ex {
                    self.draw(x, y, p);
                }
            };

            while y0 >= x0 {
                drawline(x - y0, x + y0, y - x0);
                if x0 > 0 {
                    drawline(x - y0, x + y0, y + x0);
                }

                if d < 0 {
                    d += 4 * x0 + 6;
                    x0 += 1;
                } else {
                    if x0 != y0 {
                        drawline(x - x0, x + x0, y - y0);
                        drawline(x - x0, x + x0, y + y0);
                    }
                    d += 4 * (x0 - y0) + 10;
                    x0 += 1;
                    y0 -= 1;
                }
            }
        } else {
            self.draw(x, y, p);
        }
    }
    pub fn draw_rect(&mut self, pos: Vi2d, size: Vi2d, p: Pixel) {
        self.draw_rect_xy(pos.x, pos.y, size.x, size.y, p);
    }
    pub fn draw_rect_xy(&mut self, x: i32, y: i32, w: i32, h: i32, p: Pixel) {
        self.draw_line_xy(x, y, x + w, y, p);
        self.draw_line_xy(x + w, y, x + w, y + h, p);
        self.draw_line_xy(x + w, y + h, x, y + h, p);
        self.draw_line_xy(x, y + h, x, y, p);
    }

    pub fn draw_triangle(&mut self, pos1: Vf2d, pos2: Vf2d, pos3: Vf2d, p: Pixel) {
        self.draw_triangle_xy(
            pos1.x as i32,
            pos1.y as i32,
            pos2.x as i32,
            pos2.y as i32,
            pos3.x as i32,
            pos3.y as i32,
            p,
        );
    }

    pub fn draw_triangle_xy(
        &mut self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        x3: i32,
        y3: i32,
        p: Pixel,
    ) {
        self.draw_line_xy(x1, y1, x2, y2, Pixel::GREEN);
        self.draw_line_xy(x2, y2, x3, y3, Pixel::RED);
        self.draw_line_xy(x3, y3, x1, y1, Pixel::BLUE);
    }

    pub fn fill_triangle(&mut self, mut pos1: Vf2d, mut pos2: Vf2d, mut pos3: Vf2d, p: Pixel) {
        //Sort the points so that y1 <= y2 <= y3
        if pos2.y < pos1.y {
            std::mem::swap(&mut pos2, &mut pos1);
        }
        if pos3.y < pos1.y {
            std::mem::swap(&mut pos3, &mut pos1);
        }
        if pos3.y < pos2.y {
            std::mem::swap(&mut pos3, &mut pos2);
        }
        //This takes two vectors and a y position and returns the x coordinate
        let interpolate = |l: &Vf2d, r: &Vf2d, y: f32| -> f32 {
            let m = (r.x - l.x) / (r.y - l.y);
            (m * (y - l.y)) + l.x
        };

        for y in pos1.y.ceil() as i32..pos3.y.ceil() as i32 {
            let (mut start_x, mut end_x);
            //Fill the top half of the triangle
            if y < pos2.y.ceil() as i32 {
                start_x = interpolate(&pos1, &pos2, y as f32);
                end_x = interpolate(&pos1, &pos3, y as f32);
            } else {
                start_x = interpolate(&pos2, &pos3, y as f32);
                end_x = interpolate(&pos1, &pos3, y as f32);
            }
            if end_x - start_x < 0.0 {
                std::mem::swap(&mut start_x, &mut end_x);
            }
            for x in start_x as i32..end_x as i32 {
                self.draw(x, y, p);
            }
        }
    }

    pub fn texture_triangle(
        &mut self,
        mut pos1: Vf2d,
        mut pos2: Vf2d,
        mut pos3: Vf2d,
        mut uv1: UV,
        mut uv2: UV,
        mut uv3: UV,
        spr: Option<&Sprite>,
        tp: Pixel,
    ) {
        //Sort the points so that y1 <= y2 <= y3
        if pos2.y < pos1.y {
            std::mem::swap(&mut pos2, &mut pos1);
            std::mem::swap(&mut uv2, &mut uv1);
        }
        if pos3.y < pos1.y {
            std::mem::swap(&mut pos3, &mut pos1);
            std::mem::swap(&mut uv3, &mut uv1);
        }
        if pos3.y < pos2.y {
            std::mem::swap(&mut pos3, &mut pos2);
            std::mem::swap(&mut uv3, &mut uv2);
        }

        //This takes two vectors and a y position and returns the x coordinate
        let interpolate = |l: &Vf2d, r: &Vf2d, y: f32| -> f32 {
            let m = (r.x - l.x) / (r.y - l.y);
            (m * (y - l.y)) + l.x
        };

        //This takes two UVs and a y position and returns the x coordinate
        let interpolate_uv = |l: &UV, r: &UV, p: f32| -> UV {
            let d_u = r.u - l.u;
            let d_v = r.v - l.v;
            let d_w = r.w - l.w;
            ((d_u * p) + l.u, (d_v * p) + l.v, (d_w * p) + l.w).into()
        };
        pos1.x = pos1.x.min(self.pixels_w as f32);
        pos1.x = pos1.x.max(0.0);
        pos1.y = pos1.y.min(self.pixels_h as f32);
        pos1.y = pos1.y.max(0.0);

        pos2.x = pos2.x.min(self.pixels_w as f32);
        pos2.x = pos2.x.max(0.0);
        pos2.y = pos2.y.min(self.pixels_h as f32);
        pos2.y = pos2.y.max(0.0);

        pos3.x = pos3.x.min(self.pixels_w as f32);
        pos3.x = pos3.x.max(0.0);
        pos3.y = pos3.y.min(self.pixels_h as f32);
        pos3.y = pos3.y.max(0.0);

        for y in pos1.y.ceil() as i32..pos3.y.ceil() as i32 {
            let mut p = Pixel::BLANK;
            let dy_p12 = (y as f32 - pos1.y) / (pos2.y - pos1.y);
            let dy_p23 = (y as f32 - pos2.y) / (pos3.y - pos2.y);
            let dy_p13 = (y as f32 - pos1.y) / (pos3.y - pos1.y);
            let (mut start_x, mut end_x, mut start_uv, mut end_uv);
            //Fill the top half of the triangle
            if y < pos2.y.ceil() as i32 {
                start_x = interpolate(&pos1, &pos2, y as f32);
                end_x = interpolate(&pos1, &pos3, y as f32);

                start_uv = interpolate_uv(&uv1, &uv2, dy_p12);
                end_uv = interpolate_uv(&uv1, &uv3, dy_p13);
            } else {
                start_x = interpolate(&pos2, &pos3, y as f32);
                end_x = interpolate(&pos1, &pos3, y as f32);

                start_uv = interpolate_uv(&uv2, &uv3, dy_p23);
                end_uv = interpolate_uv(&uv1, &uv3, dy_p13);
            }

            if end_x - start_x < 0.0 {
                std::mem::swap(&mut start_x, &mut end_x);
                std::mem::swap(&mut start_uv, &mut end_uv);
            }
            for x in start_x as i32..end_x as i32 {
                let r = 1.0 - ((end_x as f64 - x as f64) / (end_x as f64 - start_x as f64));
                let d_uv: UV = (
                    end_uv.u - start_uv.u,
                    end_uv.v - start_uv.v,
                    end_uv.w - start_uv.w,
                )
                    .into();
                let dw = start_uv.w as f64 + (d_uv.w as f64 * r);
                if dw > self.depth_buffer[(y * self.pixels_w as i32 + x) as usize] {
                    let mut p = Pixel::default();
                    if let Some(sp) = spr {
                        p = sp.sample(
                            ((start_uv.u as f64 + (d_uv.u as f64 * r)) / dw) as f32,
                            ((start_uv.v as f64 + (d_uv.v as f64 * r)) / dw) as f32,
                        );
                    } else {
                        p = tp;
                    }
                    self.draw(x, y, p);
                    /*if wire && (x == start_x as i32 || x == (end_x as i32 - 1)
                        || y == pos1.y.ceil() as i32 || y == (pos3.y.ceil() as i32 - 1)) {
                        self.draw(x, y, Pixel::BLACK);
                    }*/
                    self.depth_buffer[(y * self.pixels_w as i32 + x) as usize] = dw;
                }
            }
        }
    }

    pub fn clear(&mut self, p: Pixel) {
        let (h, w) = (
            self.get_draw_target_height() as u32,
            self.get_draw_target_width() as u32,
        );
        let pixels = h * w;
        let m = &mut self
            .get_draw_target()
            .expect("Can't get draw target")
            .sprite
            .col_data;
        for i in 0..pixels {
            m[i as usize] = p;
        }
    }

    fn construct_font_sheet(&mut self) {
        let mut data: String = "".to_string();
        data += "?Q`0001oOch0o01o@F40o0<AGD4090LAGD<090@A7ch0?00O7Q`0600>00000000";
        data += "O000000nOT0063Qo4d8>?7a14Gno94AA4gno94AaOT0>o3`oO400o7QN00000400";
        data += "Of80001oOg<7O7moBGT7O7lABET024@aBEd714AiOdl717a_=TH013Q>00000000";
        data += "720D000V?V5oB3Q_HdUoE7a9@DdDE4A9@DmoE4A;Hg]oM4Aj8S4D84@`00000000";
        data += "OaPT1000Oa`^13P1@AI[?g`1@A=[OdAoHgljA4Ao?WlBA7l1710007l100000000";
        data += "ObM6000oOfMV?3QoBDD`O7a0BDDH@5A0BDD<@5A0BGeVO5ao@CQR?5Po00000000";
        data += "Oc``000?Ogij70PO2D]??0Ph2DUM@7i`2DTg@7lh2GUj?0TO0C1870T?00000000";
        data += "70<4001o?P<7?1QoHg43O;`h@GT0@:@LB@d0>:@hN@L0@?aoN@<0O7ao0000?000";
        data += "OcH0001SOglLA7mg24TnK7ln24US>0PL24U140PnOgl0>7QgOcH0K71S0000A000";
        data += "00H00000@Dm1S007@DUSg00?OdTnH7YhOfTL<7Yh@Cl0700?@Ah0300700000000";
        data += "<008001QL00ZA41a@6HnI<1i@FHLM81M@@0LG81?O`0nC?Y7?`0ZA7Y300080000";
        data += "O`082000Oh0827mo6>Hn?Wmo?6HnMb11MP08@C11H`08@FP0@@0004@000000000";
        data += "00P00001Oab00003OcKP0006@6=PMgl<@440MglH@000000`@000001P00000000";
        data += "Ob@8@@00Ob@8@Ga13R@8Mga172@8?PAo3R@827QoOb@820@0O`0007`0000007P0";
        data += "O`000P08Od400g`<3V=P0G`673IP0`@3>1`00P@6O`P00g`<O`000GP800000000";
        data += "?P9PL020O`<`N3R0@E4HC7b0@ET<ATB0@@l6C4B0O`H3N7b0?P01L3R000000020";

        let mut font_sprite = Sprite::new(128, 48);
        let mut py = 0;
        let mut px = 0;
        let mut data_chars: [u8; 1024] = [0; 1024];
        for (i, c) in data.chars().enumerate() {
            data_chars[i] = c as u8;
        }
        for b in (0..1024).step_by(4) {
            let sym1: u32 = (data_chars[b + 0] as u32) - 48;
            let sym2: u32 = (data_chars[b + 1] as u32) - 48;
            let sym3: u32 = (data_chars[b + 2] as u32) - 48;
            let sym4: u32 = (data_chars[b + 3] as u32) - 48;
            let r: u32 = sym1 << 18 | sym2 << 12 | sym3 << 6 | sym4;

            for i in 0..24 {
                let k: u8 = if r & (1 << i) > 0 { 255 } else { 0 };
                font_sprite.set_pixel(px, py, Pixel::rgba(k, k, k, k));
                py += 1;
                if py == 48 {
                    px += 1;
                    py = 0;
                }
            }
        }
        //self.font_decal = Decal::create(Some(font_sprite), &mut self.renderer);
    }

    pub fn draw_decal(&mut self, pos: Vf2d, decal: Arc<SmallD>) {
        self.draw_decal_with_scale_and_tint(pos, decal, Vf2d::new(1.0, 1.0), Pixel::WHITE);
    }

    pub fn draw_decal_with_scale(&mut self, pos: Vf2d, decal: Arc<SmallD>, scale: Vf2d) {
        self.draw_decal_with_scale_and_tint(pos, decal, scale, Pixel::WHITE);
    }
    pub fn draw_decal_with_tint(&mut self, pos: Vf2d, decal: Arc<SmallD>, tint: Pixel) {
        self.draw_decal_with_scale_and_tint(pos, decal, Vf2d::new(1.0, 1.0), tint);
    }

    pub fn draw_decal_with_scale_and_tint(
        &mut self,
        pos: Vf2d,
        decal: Arc<SmallD>,
        scale: Vf2d,
        tint: Pixel,
    ) {
        let screen_space_pos = Vf2d::from((
            (pos.x * self.inv_screen_size.x) * 2.0 - 1.0,
            ((pos.y * self.inv_screen_size.y) * 2.0 - 1.0) * -1.0,
        ));
        let screen_space_dim = Vf2d::from((
            screen_space_pos.x + (2.0 * (decal.sprite.width as f32) * self.inv_screen_size.x),
            screen_space_pos.y - (2.0 * (decal.sprite.height as f32) * self.inv_screen_size.y),
        ));
        let mut di = DecalInstance::new();
        di.decal = Some(decal);
        di.tint[0] = tint;
        di.pos[0] = Vf2d::from((screen_space_pos.x, screen_space_pos.y));
        di.pos[1] = Vf2d::from((screen_space_pos.x, screen_space_dim.y));
        di.pos[2] = Vf2d::from((screen_space_dim.x, screen_space_dim.y));
        di.pos[3] = Vf2d::from((screen_space_dim.x, screen_space_pos.y));
        self.push_decal_instance(di);
        //self.get_draw_target().vec_decal_instance.push(di);
    }

    pub fn draw_partial_decal(
        &mut self,
        pos: Vf2d,
        decal: Arc<SmallD>,
        source_pos: Vf2d,
        source_size: Vf2d,
        scale: Vf2d,
        tint: Pixel,
    ) {
        let screen_space_pos = Vf2d::from((
            (pos.x * self.inv_screen_size.x) * 2.0 - 1.0,
            ((pos.y * self.inv_screen_size.y) * 2.0 - 1.0) * -1.0,
        ));
        let screen_space_dim = Vf2d::from((
            screen_space_pos.x + (2.0 * (source_size.x as f32) * self.inv_screen_size.x) * scale.x,
            screen_space_pos.y - (2.0 * (source_size.y as f32) * self.inv_screen_size.y) * scale.y,
        ));
        let mut di = DecalInstance::new();
        di.tint[0] = tint;

        di.pos[0] = Vf2d::from((screen_space_pos.x, screen_space_pos.y));
        di.pos[1] = Vf2d::from((screen_space_pos.x, screen_space_dim.y));
        di.pos[2] = Vf2d::from((screen_space_dim.x, screen_space_dim.y));
        di.pos[3] = Vf2d::from((screen_space_dim.x, screen_space_pos.y));

        let uvtl = Vf2d::from((
            source_pos.x * decal.uv_scale.x,
            source_pos.y * decal.uv_scale.y,
        ));

        let uvbr = Vf2d::from((
            uvtl.x + (source_size.x * decal.uv_scale.x),
            uvtl.y + (source_size.y * decal.uv_scale.y),
        ));

        di.uv[0] = Vf2d::from((uvtl.x, uvtl.y));
        di.uv[1] = Vf2d::from((uvtl.x, uvbr.y));
        di.uv[2] = Vf2d::from((uvbr.x, uvbr.y));
        di.uv[3] = Vf2d::from((uvbr.x, uvtl.y));
        di.decal = Some(decal);
        self.push_decal_instance(di);
    }

    pub fn draw_rotated_decal(
        &mut self,
        pos: Vf2d,
        decal: Arc<SmallD>,
        angle: f32,
        center: Vf2d,
        scale: Vf2d,
        tint: Pixel,
    ) {
        let mut di = DecalInstance::new();
        di.tint[0] = tint;
        di.pos[0] = Vf2d::new(0.0 - center.x * scale.x, 0.0 - center.y * scale.y);
        di.pos[1] = Vf2d::new(
            0.0 - center.x * scale.x,
            decal.sprite.height as f32 - center.y * scale.y,
        );
        di.pos[2] = Vf2d::new(
            decal.sprite.width as f32 - center.x * scale.x,
            decal.sprite.height as f32 - center.y * scale.y,
        );
        di.pos[3] = Vf2d::new(
            decal.sprite.width as f32 - center.x * scale.x,
            0.0 - center.y * scale.y,
        );
        let (c, s) = (angle.cos(), angle.sin());
        for i in 0..4 {
            di.pos[i] = Vf2d::new(
                di.pos[1].x * c - di.pos[i].y * s,
                di.pos[i].x * s + di.pos[i].y * c,
            );
            di.pos[i] = Vf2d::new(
                di.pos[i].x * self.inv_screen_size.x * 2.0 - 1.0,
                di.pos[i].y * self.inv_screen_size.y * 2.0 - 1.0,
            );
            di.pos[i].y += -1.0;
        }
        di.decal = Some(decal);
        self.push_decal_instance(di);
    }

    pub fn draw_warped_decal(&mut self, decal: Arc<SmallD>, pos: &[Vf2d]) {
        self.draw_warped_decal_with_tint(decal, pos, Pixel::WHITE);
    }

    pub fn draw_warped_decal_with_tint(&mut self, decal: Arc<SmallD>, pos: &[Vf2d], tint: Pixel) {
        let mut di = DecalInstance::new();
        di.decal = Some(decal);
        di.tint[0] = tint;
        let mut center = Vf2d::new(0.0, 0.0);
        let mut rd: f32 = (pos[2].x - pos[0].x) * (pos[3].y - pos[1].y)
            - (pos[3].x - pos[1].x) * (pos[2].y - pos[0].y);
        if rd != 0.0 {
            rd = 1.0 / rd;
            let rn: f32 = ((pos[3].x - pos[1].x) * (pos[0].y - pos[1].y)
                - (pos[3].y - pos[1].y) * (pos[0].x - pos[1].x))
                * rd;
            let sn: f32 = ((pos[2].x - pos[0].x) * (pos[0].y - pos[1].y)
                - (pos[2].y - pos[0].y) * (pos[0].x - pos[1].x))
                * rd;
            if !(rn < 0.0 || rn > 1.0 || sn < 0.0 || sn > 1.0) {
                let i = pos[2] - pos[0];
                center = pos[0] + Vf2d::new(rn * i.x, rn * i.y);
            }
            let mut d: [f32; 4] = [0.0; 4];
            for i in 0..4 {
                d[i] = (pos[i] - center).mag();
            }
            for i in 0..4 {
                let q = if d[i] == 0.0 {
                    1.0
                } else {
                    (d[i] + d[(i + 2) & 3]) / d[(i + 2) & 3]
                };
                di.uv[i].x *= q;
                di.uv[i].y *= q;
                di.w[i] *= q;
                di.pos[i] = Vf2d::new(
                    (pos[i].x * self.inv_screen_size.x) * 2.0 - 1.0,
                    ((pos[i].y * self.inv_screen_size.y) * 2.0 - 1.0) * -1.0,
                );
            }
            self.push_decal_instance(di);
        }
    }

    pub fn draw_partial_warped_decal(
        &mut self,
        decal: Arc<SmallD>,
        pos: Vec<Vf2d>,
        source_pos: Vf2d,
        source_size: Vf2d,
        tint: Pixel,
    ) {
        let mut di = DecalInstance::new();
        di.tint[0] = tint;
        let mut center = Vf2d::new(0.0, 0.0);
        let mut rd: f32 = (pos[2].x - pos[0].x) * (pos[3].y - pos[1].y)
            - (pos[3].x - pos[1].x) * (pos[2].y - pos[0].y);
        if rd != 0.0 {
            let uvtl = Vf2d::new(
                source_pos.x * decal.uv_scale.x,
                source_pos.y * decal.uv_scale.y,
            );

            let uvbr = Vf2d::new(
                uvtl.x + (source_size.x * decal.uv_scale.x),
                uvtl.y + (source_size.y * decal.uv_scale.y),
            );

            di.uv[0] = Vf2d::new(uvtl.x, uvtl.y);
            di.uv[1] = Vf2d::new(uvtl.x, uvbr.y);
            di.uv[2] = Vf2d::new(uvbr.x, uvbr.y);
            di.uv[3] = Vf2d::new(uvbr.x, uvtl.y);
            rd = 1.0 / rd;
            let rn: f32 = ((pos[3].x - pos[1].x) * (pos[0].y - pos[1].y)
                - (pos[3].y - pos[1].y) * (pos[0].x - pos[1].x))
                * rd;
            let sn: f32 = ((pos[2].x - pos[0].x) * (pos[0].y - pos[1].y)
                - (pos[2].y - pos[0].y) * (pos[0].x - pos[1].x))
                * rd;
            if !(rn < 0.0 || rn > 1.0 || sn < 0.0 || sn > 1.0) {
                let i = pos[2] - pos[0];
                center = Vf2d::new(pos[0].x + rn, pos[0].y + rn) * i;
            }
            let mut d: [f32; 4] = [0.0; 4];
            for i in 0..4 {
                d[i] = (pos[i] - center).mag();
            }
            for i in 0..4 {
                let q = if d[i] == 0.0 {
                    1.0
                } else {
                    (d[i] + d[(i + 2) & 3]) / d[(i + 2) & 3]
                };
                di.uv[i].x *= q;
                di.uv[i].y *= q;
                di.w[i] *= q;
                di.pos[i] = Vf2d::new(
                    (pos[i].x * self.inv_screen_size.x) * 2.0 - 1.0,
                    ((pos[i].y * self.inv_screen_size.y) * 2.0 - 1.0) * -1.0,
                );
            }
            di.decal = Some(decal);
            self.push_decal_instance(di);
        }
    }

    pub fn draw_explicit_decal(
        &mut self,
        decal: Arc<SmallD>,
        pos: Vec<Vf2d>,
        uv: Vec<Vf2d>,
        col: Vec<Pixel>,
    ) {
        let mut di = DecalInstance::new();
        if decal.id > 0 {
            di.decal = Some(decal);
        } else {
            di.decal = None;
        }

        for i in 0..4 {
            di.pos[i] = Vf2d::from((
                (pos[i].x * self.inv_screen_size.x) * 2.0 - 1.0,
                (pos[i].y * self.inv_screen_size.y) * 2.0 - 1.0,
            ));
            di.uv[i] = uv[i];
            di.tint[i] = col[i];
        }
        self.push_decal_instance(di);
    }

    pub fn fill_rect_decal(&mut self, pos: Vf2d, size: Vf2d, col: Pixel) {
        let points = vec![
            pos,
            Vf2d::new(pos.x, pos.y + size.y),
            pos + size,
            Vf2d::new(pos.x + size.x, pos.y),
        ];
        let uvs = vec![(1.0, 1.0).into(); 4];
        let cols = vec![col, col, col, col];
        self.draw_explicit_decal(Decal::empty().get(), points, uvs, cols);
    }

    pub fn gradient_fill_rect_decal(
        &mut self,
        pos: Vf2d,
        size: Vf2d,
        colTL: Pixel,
        colBL: Pixel,
        colTR: Pixel,
        colBR: Pixel,
    ) {
        let points = vec![
            pos,
            Vf2d::new(pos.x, pos.y + size.y),
            pos + size,
            Vf2d::new(pos.x + size.x, pos.y),
        ];
        let uvs = vec![(1.0, 1.0).into(); 4];
        let cols = vec![colTL, colBL, colBR, colTR];
        self.draw_explicit_decal(Decal::empty().get(), points, uvs, cols);
    }

    pub fn draw_partial_rotated_decal(
        &mut self,
        pos: Vf2d,
        decal: Arc<SmallD>,
        angle: f32,
        center: Vf2d,
        source_pos: Vf2d,
        source_size: Vf2d,
        scale: Vf2d,
        tint: Pixel,
    ) {
        let mut di = DecalInstance::new();
        di.tint[0] = tint;
        di.pos[0] = (Vf2d::new(0.0, 0.0) - center) * scale;
        di.pos[1] = (Vf2d::new(0.0, source_size.y) - center) * scale;
        di.pos[2] = (Vf2d::new(source_size.x, source_size.y) - center) * scale;
        di.pos[3] = (Vf2d::new(source_size.x, 0.0) - center) * scale;
        let (c, s) = (angle.cos(), angle.sin());
        for i in 0..4 {
            di.pos[i] = Vf2d::new(
                di.pos[1].x * c - di.pos[i].y * s,
                di.pos[i].x * s + di.pos[i].y * c,
            );
            di.pos[i] = Vf2d::new(
                di.pos[i].x * self.inv_screen_size.x * 2.0 - 1.0,
                di.pos[i].y * self.inv_screen_size.y * 2.0 - 1.0,
            );
            di.pos[i].y += -1.0;
        }
        let uvtl = Vf2d::new(
            source_pos.x * decal.uv_scale.x,
            source_pos.y * decal.uv_scale.y,
        );

        let uvbr = Vf2d::new(
            uvtl.x + (source_size.x * decal.uv_scale.x),
            uvtl.y + (source_size.y * decal.uv_scale.y),
        );

        di.uv[0] = Vf2d::new(uvtl.x, uvtl.y);
        di.uv[1] = Vf2d::new(uvtl.x, uvbr.y);
        di.uv[2] = Vf2d::new(uvbr.x, uvbr.y);
        di.uv[3] = Vf2d::new(uvbr.x, uvtl.y);
        di.decal = Some(decal);
        self.push_decal_instance(di);
    }
    pub fn draw_string_decal(&mut self, pos: Vf2d, text: &str) {
        self.draw_string_decal_with_color_and_scale(pos, text, Pixel::WHITE, Vf2d::new(1.0, 1.0));
    }

    pub fn draw_string_decal_with_color(&mut self, pos: Vf2d, text: &str, col: Pixel) {
        self.draw_string_decal_with_color_and_scale(pos, text, col, Vf2d::new(1.0, 1.0));
    }

    pub fn draw_string_decal_with_scale(&mut self, pos: Vf2d, text: &str, scale: Vf2d) {
        self.draw_string_decal_with_color_and_scale(pos, text, Pixel::WHITE, scale);
    }

    pub fn draw_string_decal_with_color_and_scale(
        &mut self,
        pos: Vf2d,
        text: &str,
        col: Pixel,
        scale: Vf2d,
    ) {
        //self.draw_decal(pos, &FONT_DECAL, scale, Pixel::WHITE);
        let mut spos = Vf2d::new(0.0, 0.0);
        for c in text.chars() {
            if c == '\n' {
                spos.x = 0.0;
                spos.y += 8.0 * scale.y;
            } else {
                let ox = (c as u8 - 32) % 16;
                let oy = (c as u8 - 32) / 16;
                self.draw_partial_decal(
                    pos + spos,
                    self.font_decal.get(),
                    Vf2d::new(ox as f32 * 8.0, oy as f32 * 8.0),
                    Vf2d::new(8.0, 8.0),
                    scale,
                    col,
                );
                spos.x += 8.0 * scale.x;
            }
        }
    }

    pub fn get_text_size(&self, s: String) -> Vi2d {
        let (mut size, mut pos) = (Vi2d::new(0, 1), Vi2d::new(0, 1));
        for c in s.chars() {
            if c == '\n' {
                pos.y += 1;
                pos.x = 0;
            } else {
                pos.x += 1
            }
            size.x = std::cmp::max(size.x, pos.x);
            size.y = std::cmp::max(size.y, pos.y);
        }
        Vi2d::new(size.x * 8, size.y * 8)
    }
}

pub trait Olc {
    fn on_engine_start(&mut self, engine: &mut OLCEngine) -> bool;

    fn on_engine_update(&mut self, engine: &mut OLCEngine, elapsedTime: f64) -> bool;

    fn on_engine_destroy(&mut self) -> bool;
}

pub trait App: Olc {}

pub async fn construct<T: 'static + App>(
    mut app: T,
    app_name: &'static str,
    screen_width: u32,
    screen_height: u32,
    pixel_width: u32,
    pixel_height: u32,
    full_screen: bool,
    vsync: bool,
) {
    //Set the olc object to be used in this crate
    unsafe {
        PLATFORM_DATA.init();
        PLATFORM_DATA.resolution = Some(Vi2d::from((
            (screen_width / pixel_width) as i32,
            (screen_height / pixel_height) as i32,
        )));
        if !full_screen {
            PLATFORM_DATA.window_size =
                Some(Vi2d::from((screen_width as i32, screen_height as i32)));
        }
        PLATFORM_DATA.full_screen = full_screen;
        PLATFORM_DATA.title = app_name.into();
        PLATFORM_DATA.pixel_size = Some(Vi2d::new(pixel_width as i32, pixel_height as i32));
    };
    let (mut window, mut event_loop) = PlatformWindows::create_window_pane(
        Vi2d { x: 10, y: 10 },
        unsafe { PLATFORM_DATA.window_size.unwrap() },
        unsafe { PLATFORM_DATA.full_screen },
    );

    #[cfg(target_arch = "wasm32")]
    {
        log::trace!("building canvas");
        use winit::platform::web::WindowExtWebSys;

        let canvas = window.canvas();

        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let body = document.body().unwrap();
        body.append_child(&canvas)
            .expect("Append canvas to HTML body");
    }

    let mut renderer: Renderer = Renderer::new(&window).await;

    let mut engine = OLCEngine {
        app_name: String::from(""),
        is_focused: true,
        window_width: 0,
        window_height: 0,
        pixels_w: 0,
        pixels_h: 0,
        pixel_width: 0,
        pixel_height: 0,
        inv_screen_size: Vf2d::new(0.0, 0.0),
        fps: 0,
        full_screen: false,
        renderer,
        vsync: false,
        layers: vec![],
        draw_target: 0,
        mouse_position: Vi2d::from((0, 0)),
        font_decal: Decal::empty(),
        depth_buffer: vec![],
        camera: camera::Camera::default(),
        window,
    };
    engine.init(
        app_name,
        screen_width,
        screen_height,
        pixel_width,
        pixel_height,
        full_screen,
        vsync,
    );
    start_game(app, engine, event_loop).await;
}

pub async fn start_game<T: 'static + Olc>(
    mut app: T,
    mut engine: OLCEngine,
    mut event_loop: EventLoop<()>,
) {
    unsafe {
        if PLATFORM_DATA.full_screen {
            let fwin = PLATFORM_DATA.window_size.unwrap_or_default().to_vf2d();
            let fres = PLATFORM_DATA.resolution.unwrap_or_default().to_vf2d();
            PLATFORM_DATA.pixel_size = Some(
                (
                    (fwin.x as f32 / fres.x as f32) as i32,
                    (fwin.y as f32 / fres.y as f32) as i32,
                )
                    .into(),
            );
        }
        //(GL.glLoadIdentity)();
    }
    engine.construct_font_sheet();
    //Create Primary Layer "0"
    let base_layer_id = engine.add_layer();
    let base_layer = engine.get_layer(base_layer_id).unwrap();
    engine.set_draw_target(base_layer_id);

    engine.renderer.setup_layer_pipeline();
    //engine.renderer.setup_3D_pipeline();

    let mut frame_timer: f64 = 0.0;
    let mut frame_count: i32 = 0;
    let mut last_fps: i32 = 0;
    #[cfg(not(target_arch = "wasm32"))]
    let mut game_timer = UNIX_EPOCH.elapsed().unwrap().as_secs_f64();

    #[cfg(target_arch = "wasm32")]
    let mut game_timer = js_sys::Date::now() as f64;
    /*Renderer::update_texture(engine.get_draw_target_ref().id,
    &engine.get_draw_target_ref().sprite);*/

    //game_engine.construct_font_sheet();
    if app.on_engine_start(&mut engine) {
        event_loop.run(move |top_event, window_target, control_flow| {
            *control_flow = ControlFlow::Poll;

            match top_event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                } => {
                    if window_id == engine.window.id() {
                        if event == &WindowEvent::CloseRequested {
                            *control_flow = ControlFlow::Exit;
                        } else {
                            PlatformWindows::handle_window_event(&engine.window, &top_event);
                        }
                    }
                }
                Event::RedrawRequested(_) => {
                    engine.renderer.active_decals = vec![];
                    for layer in engine.layers.iter_mut() {
                        if layer.shown {
                            engine
                                .renderer
                                .active_decals
                                .insert(layer.id as usize, layer.id as i32);
                        }
                        if layer.update {
                            engine
                                .renderer
                                .update_texture(layer.id as u32, &layer.sprite);
                            layer.update = false;
                        }
                    }
                    let mut encoder = engine.renderer.device.create_command_encoder(
                        &wgpu::CommandEncoderDescriptor {
                            label: Some("Render Encoder"),
                        },
                    );
                    engine.renderer.update_texture_groups();
                    //engine.renderer.draw_3D(&engine.camera, &mut encoder);
                    engine.renderer.draw_points(&engine.camera, &mut encoder);
                    engine.renderer.draw_layers(&mut encoder);
                    // submit will accept anything that implements IntoIter
                    engine
                        .renderer
                        .queue
                        .submit(std::iter::once(encoder.finish()));
                    engine.renderer.clear_frame();
                }
                _ => {
                    //#[cfg(not(target_arch = "wasm32"))]
                    PlatformWindows::handle_window_event(&engine.window, &top_event);
                }
            }

            #[cfg(not(target_arch = "wasm32"))]
            let elapsed_time = (UNIX_EPOCH.elapsed().unwrap().as_secs_f64() - game_timer);

            #[cfg(target_arch = "wasm32")]
            let elapsed_time = (js_sys::Date::now() as f64 - game_timer) / 1000.0;

            #[cfg(not(target_arch = "wasm32"))]
            {
                game_timer = UNIX_EPOCH.elapsed().unwrap().as_secs_f64();
            }

            #[cfg(target_arch = "wasm32")]
            {
                game_timer = js_sys::Date::now() as f64;
            }
            unsafe {
                let hw_func = |keys: &mut Vec<HWButton>,
                               keys_old: &mut Vec<bool>,
                               keys_new: &mut Vec<bool>,
                               size: usize| {
                    for i in 0..size as usize {
                        keys[i].pressed = false;
                        keys[i].released = false;
                        if keys_new[i] != keys_old[i] {
                            if keys_new[i] {
                                keys[i].pressed = true;
                                keys[i].released = false;
                                keys[i].held = true;
                            } else {
                                keys[i].pressed = false;
                                keys[i].released = true;
                                keys[i].held = false;
                            }
                        }
                        keys_old[i] = keys_new[i];
                    }
                };
                engine.clear_keys();
                for (key, value_new) in PLATFORM_DATA.new_key_state_map.as_mut().unwrap() {
                    let value_old = PLATFORM_DATA
                        .old_key_state_map
                        .as_mut()
                        .unwrap()
                        .entry(*key)
                        .or_insert(false);
                    let current_key = PLATFORM_DATA
                        .key_map
                        .as_mut()
                        .unwrap()
                        .entry(*key)
                        .or_insert_with(HWButton::new);
                    if value_new != value_old {
                        if *value_new {
                            (*current_key).pressed = true;
                            (*current_key).released = false;
                            (*current_key).held = true;
                        } else {
                            (*current_key).pressed = false;
                            (*current_key).released = true;
                            (*current_key).held = false;
                        }
                    }
                    *value_old = *value_new
                }

                hw_func(
                    PLATFORM_DATA.mouse_map.as_mut().unwrap(),
                    PLATFORM_DATA.old_mouse_state_map.as_mut().unwrap(),
                    PLATFORM_DATA.new_mouse_state_map.as_mut().unwrap(),
                    3,
                );

                if let Some(pos) = PLATFORM_DATA.mouse_position_cache {
                    PLATFORM_DATA.mouse_position = Some(pos);
                }
                PLATFORM_DATA.mouse_wheel_delta = PLATFORM_DATA.mouse_wheel_delta_cache;
                PLATFORM_DATA.mouse_wheel_delta_cache = 0;
            }
            engine.renderer.new_frame();
            if !app.on_engine_update(&mut engine, elapsed_time) {
                *control_flow = ControlFlow::Exit;
            }
            unsafe {
                engine.renderer.update_viewport(
                    Vi2d { x: 0, y: 0 },
                    Vi2d {
                        x: PLATFORM_DATA.window_size.unwrap().x,
                        y: PLATFORM_DATA.window_size.unwrap().y,
                    },
                );
            }
            //renderer.clear_buffer(Pixel::rgba(0, 0, 0, 255), true);
            //always draw the background
            engine.layers[0].update = true;
            engine.layers[0].shown = true;
            //Renderer::prepare_drawing();

            /*let layer_iter = engine.layers.iter_mut();

            for layer in layer_iter {
                if layer.shown {
                    match layer.func_hook {
                        None => {
                            Renderer::apply_texture(layer.id);
                            Renderer::draw_layer_quad(layer.offset,
                                                      layer.scale,
                                                      layer.tint);
                            if !layer.vec_decal_instance.is_empty() {
                                let layer_decals = layer.vec_decal_instance.iter_mut();
                                for decal in layer_decals {
                                    Renderer::draw_decal_quad(decal);
                                }
                                layer.vec_decal_instance.clear();
                            }
                        }
                        Some(function) =>
                        //Run the custom function hook
                            function(layer)
                    }
                }
            }*/
            engine.window.request_redraw();
            // Update Title Bar
            frame_timer += elapsed_time;
            frame_count += 1;
            if frame_timer >= 1.0 {
                last_fps = frame_count;
                engine.fps = (frame_count as f64 / frame_timer).floor() as u32;
                let sTitle = engine.app_name.to_string() + " - FPS: " + &(engine.fps).to_string();
                PlatformWindows::set_window_title(&engine.window, sTitle);
                frame_count = 0;
                frame_timer -= 1.0;
            }
        });
    };
    unsafe {
        PLATFORM_DATA.running = false;
    }
    //self.on_engine_destroy();
}

impl<T> App for T where T: Olc {}

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

    pub fn to_vf2d(&self) -> Vf2d {
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

    pub fn to_vi2d(&self) -> Vi2d {
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

impl HWButton {
    fn new() -> Self {
        HWButton {
            pressed: false,
            released: false,
            held: false,
        }
    }
}

macro_rules! map (
    { $($key:expr => $value:expr),+ } => {
        {
            let m = ::std::collections::HashMap::new();
            $(
                m.insert($key, $value);
            )+
            m
        }
     };
);
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mouse {
    LEFT,
    RIGHT,
    MIDDLE,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V2d<T> {
    pub x: T,
    pub y: T,
}

pub type Vi2d = V2d<i32>;
pub type Vf2d = V2d<f32>;

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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

#[derive(Clone, Default)]
pub struct Sprite {
    mode_sample: SpriteMode,
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

    pub fn set_pixel(&mut self, x: u32, y: u32, p: Pixel) -> bool {
        if x < self.width && y < self.height {
            self.col_data[(y * self.width + x) as usize] = p;
            true
        } else {
            false
        }
    }
    pub fn set_region(&mut self, x: u32, y: u32, width: u32, height: u32, p: &[Pixel]) -> bool {
        if (x + width) < self.width && (y + height) < self.height {
            Renderer::update_texture_region(0, x, y, width, height, &p);
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

    pub fn sample_bl(u: f32, v: f32) -> Pixel {
        Pixel::rgb(0, 0, 0)
    }

    pub fn get_data(&self) -> &[u8] {
        let p_ptr = self.col_data.as_slice() as *const _ as *const u8;
        unsafe { std::slice::from_raw_parts(p_ptr, self.col_data.len() * 4) }
    }

    /*pub fn as_texture(&self) -> u32{
        let tex_id = Renderer::create_texture(self.width, self.height);
        Renderer::apply_texture(tex_id);
        Renderer::update_texture(0, self);
        tex_id
    }*/

    pub fn overwrite_from_file<T: ImageLoader>(&mut self, file_path: &str) -> Rcode {
        T::load_image_resource(self, file_path)
    }

    pub fn load_from_file<T: ImageLoader>(file_path: &str) -> Option<Self> {
        let mut spr = Sprite::new(0, 0);
        match T::load_image_resource(&mut spr, file_path) {
            Rcode::Ok => Some(spr),
            _ => None,
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

#[derive(Clone)]
pub struct SmallD {
    pub id: i32,
    pub sprite: Sprite,
    uv_scale: Vf2d,
}

#[derive(Clone)]
pub struct Decal {
    d_inst: Arc<SmallD>,
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

impl DecalInstance {
    pub fn new() -> Self {
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

#[derive(Clone)]
pub struct LayerDesc {
    pub id: u32,
    pub offset: Vf2d,
    pub scale: Vf2d,
    pub tint: Pixel,
    pub shown: bool,
    pub sprite: Sprite,
    pub update: bool,
    pub vec_decal_instance: Vec<DecalInstance>,
    pub func_hook: Option<fn(&mut LayerDesc)>,
}

impl LayerDesc {
    pub fn empty() -> Self {
        LayerDesc {
            id: 0,
            offset: Default::default(),
            scale: Default::default(),
            tint: Pixel::rgb(0, 0, 0),
            shown: false,
            sprite: Sprite::new(0, 0),
            update: false,
            vec_decal_instance: vec![],
            func_hook: None,
        }
    }

    pub fn new(tex_w: u32, tex_h: u32) -> Self {
        LayerDesc {
            id: 0,
            offset: Vf2d { x: 0.0, y: 0.0 },
            scale: Vf2d { x: 1.0, y: 1.0 },
            tint: Pixel::rgba(255, 255, 255, 255),
            shown: false,
            update: false,
            sprite: Sprite::new(tex_w, tex_h),
            vec_decal_instance: vec![],
            func_hook: None,
        }
    }
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
        let (mut r, mut g, mut b, mut a, mut i): (u8, u8, u8, u8, u8) = (0x00, 0x00, 0x00, 0x00, 0);
        for p in colors.iter() {
            if p != &Pixel::BLANK {
                unsafe {
                    r += p.rgba.0;
                    g += p.rgba.1;
                    b += p.rgba.2;
                    a += p.rgba.3;
                }
                i += 1;
            }
        }
        if i > 0 {
            Pixel::rgba(r / i as u8, g / i as u8, b / i as u8, a)
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

pub trait ImageLoader {
    fn load_image_resource(spr: &mut Sprite, image_file: &str) -> Rcode;
    fn save_image_resource(spr: Sprite, image_file: &str) -> Rcode;
}

pub struct BMPLoader;
pub struct PNGLoader;

pub struct ResourceBuffer {}

pub struct ResourcePack {}

impl ImageLoader for BMPLoader {
    fn load_image_resource(spr: &mut Sprite, image_file: &str) -> Rcode {
        let image_path = std::path::Path::new(image_file);
        if !image_path.exists() {
            return Rcode::NoFile;
        }
        spr.col_data.clear();
        let img = bmp::open(image_path).unwrap_or_else(|e| bmp::Image::new(0, 0));
        if img.get_width() == 0 || img.get_height() == 0 {
            return Rcode::Fail;
        }
        spr.width = img.get_width();
        spr.height = img.get_height();
        //No Alpha for now because BMP is a dumb format
        spr.col_data = vec![Pixel::rgb(0, 0, 0); (spr.width * spr.height) as usize];
        for y in 0..spr.height {
            for x in 0..spr.width {
                let p = img.get_pixel(x, y);
                spr.set_pixel(x, y, Pixel::rgb(p.r, p.g, p.b));
            }
        }
        Rcode::Ok
    }
    fn save_image_resource(spr: Sprite, image_file: &str) -> Rcode {
        Rcode::Ok
    }
}

impl ImageLoader for PNGLoader {
    fn load_image_resource(spr: &mut Sprite, image_file: &str) -> Rcode {
        let image_path = std::path::Path::new(image_file);
        if !image_path.exists() {
            return Rcode::NoFile;
        }
        spr.col_data.clear();
        let mut img = png::Decoder::new(File::open(image_path).unwrap());
        img.set_transformations(Transformations::EXPAND);
        let (img_info, mut reader) = img.read_info().unwrap();
        if img_info.width == 0 || img_info.height == 0 {
            return Rcode::Fail;
        }
        let mut buf = vec![0; img_info.buffer_size()];
        spr.width = img_info.width;
        spr.height = img_info.height;
        reader.next_frame(&mut buf).unwrap();
        spr.col_data = vec![Pixel::rgba(0, 0, 0, 0); (spr.width * spr.height) as usize];
        for y in 0..spr.height as usize {
            for x in 0..spr.width as usize {
                if img_info.color_type == png::ColorType::RGB {
                    let index = y * (spr.width as usize * 3) + (x * 3);
                    spr.set_pixel(
                        x as u32,
                        y as u32,
                        Pixel::rgb(buf[index], buf[index + 1], buf[index + 2]),
                    );
                } else if img_info.color_type == png::ColorType::RGBA {
                    let index = y * (spr.width as usize * 4) + (x * 4);
                    spr.set_pixel(
                        x as u32,
                        y as u32,
                        Pixel::rgba(buf[index], buf[index + 1], buf[index + 2], buf[index + 3]),
                    );
                }
            }
        }
        Rcode::Ok
    }
    fn save_image_resource(spr: Sprite, image_file: &str) -> Rcode {
        Rcode::Ok
    }
}

/*
pub fn check_gl_error(i: i32) {
    let mut a = (GL.glGetError)();
    let mut errs = vec![];
    while a != 0 {
        errs.push(a);
        a = (GL.glGetError)();
    }
    if !errs.is_empty() {
        println!("Errors: {:?}, Position: {} ",
                 errs, i)
    }
}

pub type GLCallback = fn(source: u32, m_type: u32, id: u32, severity: u32,
                         length: u32, message: *const std::os::raw::c_char,
                         userParam: *const usize);

pub fn gl_message_callback(source: u32, m_type: u32, id: u32, severity: u32,
                           length: u32, message: *const std::os::raw::c_char,
                           userParam: *const usize) {
    unsafe {
        println!("GL CALLBACK: {} type = {:#X}, severity = {:#X}, message = {}",
                 (if m_type == 0x824C { "** GL ERROR **" } else { "" }),
                 m_type, severity, CStr::from_ptr(message).to_str().unwrap_or_default())
    }
}

macro_rules! gl_function {
    ($func_name:ident $(,$x:ty)* $(| $y:ty)*) => {
        unsafe{
            let glp = GLLoader::get_function_pointer(stringify!($func_name));
            let func: extern "C" fn ($($x),*) $(-> $y)* =
            std::mem::transmute(glp);
            func
        }
    };
}
macro_rules! gl_define {
    ($($x:ty,)* $(| $y:ty)*) => {
            extern "C" fn ($($x),*) $(-> $y)*
    };
}

pub struct GLLoader {
    wglSwapIntervalEXT: gl_define!( i32,),
    glEnable: gl_define!( u32,),
    glHint: gl_define!( u32, u32,),
    glViewport: gl_define!( u32, u32, u32, u32,),
    glClearColor: gl_define!( f32, f32, f32, f32,),
    glClear: gl_define!( u32,),
    glBlendFunc: gl_define!( u32, u32,),
    glGenTextures: gl_define!( u32, &mut u32,),
    glBindTexture: gl_define!( u32, u32,),
    glTexParameteri: gl_define!( u32, u32, u32,),
    glTexEnvi: gl_define!( u32, u32, u32,),
    glTexImage2D: gl_define!( u32, u32, u32, u32, u32, u32, u32, u32, *const usize,),
    glTexSubImage2D: gl_define!( u32, u32, u32, u32, u32, u32, u32, u32, *const usize,),
    glBegin: gl_define!( u32,),
    glTranslatef: gl_define!( f32, f32, f32,),
    glLoadIdentity: gl_define!(),
    glFrustum: gl_define!(f64, f64, f64, f64, f64, f64,),
    glColor4ub: gl_define!(u8, u8, u8, u8,),
    glTexCoord2f: gl_define!( f32, f32,),
    glVertex2f: gl_define!( f32, f32,),
    glTexCoord3f: gl_define!( f32, f32, f32,),
    glVertex3f: gl_define!( f32, f32, f32,),
    glDebugMessageCallback: gl_define!( GLCallback, u32,),
    glGetError: gl_define!(| i32),
    glEnd: gl_define!(),
    glDeleteTextures: gl_define!(u32, &mut u32,),
    glTexCoord4f: gl_define!(f32, f32, f32, f32,),
}

impl GLLoader {
    pub fn construct() -> Self {
        GLLoader {
            wglSwapIntervalEXT: gl_function!(wglSwapIntervalEXT, i32),
            glEnable: gl_function!(glEnable, u32),
            glHint: gl_function!(glHint, u32, u32),
            glViewport: gl_function!(glViewport, u32, u32, u32, u32),
            glClearColor: gl_function!(glClearColor, f32, f32, f32, f32),
            glClear: gl_function!(glClear, u32),
            glBlendFunc: gl_function!(glBlendFunc, u32, u32),
            glGenTextures: gl_function!(glGenTextures, u32, &mut u32),
            glBindTexture: gl_function!(glBindTexture, u32, u32),
            glTexParameteri: gl_function!(glTexParameteri, u32, u32, u32),
            glTexEnvi: gl_function!(glTexEnvi, u32, u32, u32),
            glTexImage2D: gl_function!(glTexImage2D, u32, u32, u32, u32, u32, u32, u32, u32, *const usize),
            glTexSubImage2D: gl_function!(glTexSubImage2D, u32, u32, u32, u32, u32, u32, u32, u32, *const usize),
            glBegin: gl_function!(glBegin, u32),
            glEnd: gl_function!(glEnd),
            glTranslatef: gl_function!(glTranslatef, f32, f32, f32),
            glFrustum: gl_function!(glFrustum, f64, f64, f64, f64, f64, f64),
            glLoadIdentity: gl_function!(glLoadIdentity),
            glColor4ub: gl_function!(glColor4ub,u8, u8, u8, u8),
            glTexCoord2f: gl_function!(glTexCoord2f, f32, f32),
            glVertex2f: gl_function!(glVertex2f, f32, f32),
            glTexCoord3f: gl_function!(glTexCoord3f, f32, f32, f32),
            glVertex3f: gl_function!(glVertex3f, f32, f32, f32),
            glDebugMessageCallback: gl_function!(glDebugMessageCallback, GLCallback, u32),
            glGetError: gl_function!(glGetError | i32),
            glDeleteTextures: gl_function!(glDeleteTextures, u32, &mut u32),
            glTexCoord4f: gl_function!(glTexCoord4f, f32, f32, f32, f32)
        }
    }

    pub fn get_function_pointer(func_name: &str) -> *const u64 {
        unsafe {
            let (s_func, s_ogl) =
                (CString::new(func_name).expect( "Failed to get OpenGL function"),
               CString::new("opengl32.dll").expect( "Failed to load OpenGL DLL"));
            let mut glp = wglGetProcAddress(s_func.as_ptr());
            if glp.is_null() {
                let module: HMODULE = LoadLibraryA(s_ogl.as_ptr());
                glp = GetProcAddress(module, s_func.as_ptr());
            }
            if glp.is_null() {
                println!("FAILED TO LOAD OPENGL FUNCTION: {}", func_name);
            }
            //println!("{}: {:#X}", func_name, glp as u64);
            glp as *const u64
        }
    }
}*/
