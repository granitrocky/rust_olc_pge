use super::{
    camera::{Camera, RawMat},
    decal::DecalInstance,
    game_object::GameObject,
    geometry::{Mesh, Primitives, Triangle, Vertex},
    layer::{DrawData, LayerMask, Mask},
    math_3d::Vector3,
    pixel::Pixel,
    platform::PLATFORM_DATA,
    sprite::Sprite,
    texture::Texture,
    util::{Vf2d, Vi2d},
    olc::Rcode,
};
use wgpu::util::DeviceExt;

#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowBuilderExtWebSys;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

pub const VERT_BUFFER_SIZE: usize = 25 /*MB*/ * 1024 * 1024 / std::mem::size_of::<Vertex>();
pub const MAX_VERTICES: usize = VERT_BUFFER_SIZE;
pub const INDEX_BUFFER_SIZE: usize = 5 /*MB*/ * 1024 * 1024 / std::mem::size_of::<u32>();

pub struct Renderer {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub sc_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub render_pipeline: Option<wgpu::RenderPipeline>,
    pub render_3D_pipeline_indexed: Option<wgpu::RenderPipeline>,
    pub decal_buffer: wgpu::Buffer,
    pub decals: Vec<Texture>,
    pub active_decals: Vec<u32>,
    pub decal_counter: i32,
    pub layer_textures: Option<Vec<wgpu::BindGroup>>,
    pub texture_sampler: Option<wgpu::Sampler>,
    pub bind_group_layout: Option<wgpu::BindGroupLayout>,
    pub bind_group: Option<wgpu::BindGroup>,
    pub draw_data: Vec<DrawData>,
    pub frame: Option<wgpu::SwapChainFrame>,
    pub layer_shader: wgpu::ShaderModule,
    pub indexed_vert_shader: wgpu::ShaderModule,
    pub camera_buffer: wgpu::Buffer,
    pub cam_sampler_uniform_group: Option<wgpu::BindGroup>,
    pub game_objects: Vec<GameObject>,
    pub meshes: Vec<Mesh>,
    pub vertex_buffer: wgpu::Buffer,
    pub tri_count: u32,
    pub index_count: u32,
    pub indexed_vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub frame_texture_backbuffer: Texture,
    pub frame_texture: Texture,
    pub depth_texture: Texture,
    pub default_texture: Texture,
    pub default_texture_bind: wgpu::BindGroup,
    pub camera: Camera,
}

impl Renderer {
    pub async fn new(window: &winit::window::Window) -> Self {
        let size = window.inner_size();
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU

        #[cfg(not(target_arch = "wasm32"))]
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);

        #[cfg(target_arch = "wasm32")]
        let instance = wgpu::Instance::new(wgpu::BackendBit::all());

        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("No adapter available");

        let adapter_info = adapter.get_info();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    #[cfg(target_arch = "wasm32")]
                    features: wgpu::Features::default(),
                    #[cfg(not(target_arch = "wasm32"))]
                    features: wgpu::Features::NON_FILL_POLYGON_MODE,
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None, // Trace path
            )
            .await
            .expect("No device available");

        #[cfg(target_arch = "wasm32")]
        log::warn!("DEVICE: {:?}", adapter_info);

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: adapter.get_swap_chain_preferred_format(&surface).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Immediate,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let layer_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("layer_shader"),
            flags: wgpu::ShaderFlags::all(),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/layer.wgsl").into()),
        });

        let indexed_vert_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("indexed_vert_shader"),
            flags: wgpu::ShaderFlags::all(),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/indexed.wgsl").into()),
        });

        let decal_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Decal Buffer"),
            usage: wgpu::BufferUsage::VERTEX,
            #[cfg(not(target_arch = "wasm32"))]
            contents: bytemuck::cast_slice(&[
                Vertex {
                    position: [1.0, 1.0, 0.0].into(),
                    tex_coords: [1.0, 0.0, 0.0].into(),
                    normal: Vector3::default(),
                    color: Pixel::WHITE,
                },
                Vertex {
                    position: [-1.0, 1.0, 0.0].into(),
                    tex_coords: [0.0, 0.0, 0.0].into(),
                    normal: Vector3::default(),
                    color: Pixel::WHITE,
                },
                Vertex {
                    position: [1.0, -1.0, 0.0].into(),
                    tex_coords: [1.0, 1.0, 0.0].into(),
                    normal: Vector3::default(),
                    color: Pixel::WHITE,
                },
                Vertex {
                    position: [-1.0, -1.0, 0.0].into(),
                    tex_coords: [0.0, 1.0, 0.0].into(),
                    normal: Vector3::default(),
                    color: Pixel::WHITE,
                },
                Vertex {
                    position: [1.0, -1.0, 0.0].into(),
                    tex_coords: [1.0, 1.0, 0.0].into(),
                    normal: Vector3::default(),
                    color: Pixel::WHITE,
                },
                Vertex {
                    position: [-1.0, 1.0, 0.0].into(),
                    tex_coords: [0.0, 0.0, 0.0].into(),
                    normal: Vector3::default(),
                    color: Pixel::WHITE,
                },
            ]),
            #[cfg(target_arch = "wasm32")]
            contents: bytemuck::cast_slice(&[
                Vertex {
                    position: [1.0, 1.0, 0.0].into(),
                    tex_coords: [1.0, 1.0, 0.0].into(),
                    normal: Vector3::default(),
                    color: Pixel::WHITE,
                },
                Vertex {
                    position: [-1.0, 1.0, 0.0].into(),
                    tex_coords: [0.0, 1.0, 0.0].into(),
                    normal: Vector3::default(),
                    color: Pixel::WHITE,
                },
                Vertex {
                    position: [1.0, -1.0, 0.0].into(),
                    tex_coords: [1.0, 0.0, 0.0].into(),
                    normal: Vector3::default(),
                    color: Pixel::WHITE,
                },
                Vertex {
                    position: [-1.0, -1.0, 0.0].into(),
                    tex_coords: [0.0, 0.0, 0.0].into(),
                    normal: Vector3::default(),
                    color: Pixel::WHITE,
                },
                Vertex {
                    position: [1.0, -1.0, 0.0].into(),
                    tex_coords: [1.0, 0.0, 0.0].into(),
                    normal: Vector3::default(),
                    color: Pixel::WHITE,
                },
                Vertex {
                    position: [-1.0, 1.0, 0.0].into(),
                    tex_coords: [0.0, 1.0, 0.0].into(),
                    normal: Vector3::default(),
                    color: Pixel::WHITE,
                },
            ]),
        });
        // let default_texture_data = super::util::get_file_as_u8("./tex.png").await;
        // use super::util::ImageLoader;
        // let spr: Sprite =
        //     super::util::PNGLoader::load_image_from_bytes(default_texture_data.as_slice()).unwrap();
        let (w, h) = (1024, 1024);
        let mut spr: Sprite = Sprite::new(w, h);
        for x in 0..w as usize {
            for y in 0..h as usize {
                if x % 2 == 0 || y % 2 == 0 {
                    spr.col_data[y * h as usize + x] = Pixel::rgb(150, 150, 0);
                } else {
                    spr.col_data[y * h as usize + x] = Pixel::rgb(0, 150, 150);
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        let mut default_texture = Texture::new(&device, spr.width, spr.height, sc_desc.format);
        #[cfg(not(target_arch = "wasm32"))]
        let mut default_texture = Texture::new(
            &device,
            spr.width,
            spr.height,
            wgpu::TextureFormat::Rgba8Unorm,
        );

        default_texture.update(&queue, &spr);
        let default_texture_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(
                    &default_texture.texture_bundle.as_ref().unwrap().view,
                ),
            }],
            label: None,
            layout: &device
                .create_bind_group_layout(&super::layer::DrawData::default_bind_group_layout()),
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
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(Primitives::cube().vertices().as_slice()),
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        });
        let blank_buff: Vec<Vertex> = vec![Vertex::default(); VERT_BUFFER_SIZE];
        let indexed_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Indexed Vertex Buffer"),
            //This is the maximum number of vertices that can be indexed with a u16
            contents: bytemuck::cast_slice(blank_buff.as_slice()),
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        });

        let blank_buff: Vec<u32> = vec![0; INDEX_BUFFER_SIZE];
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(blank_buff.as_slice()),
            usage: wgpu::BufferUsage::INDEX | wgpu::BufferUsage::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        comparison: false,
                        filtering: false,
                    },
                    count: None,
                },
            ],
        });
        let mut cam_data: Vec<u8> = bytemuck::cast_slice(&[RawMat::default()]).into();
        let window_size = unsafe { PLATFORM_DATA.window_size.as_ref().unwrap() };
        unsafe {
            let x_bytes: [u8; 4] = std::mem::transmute(window_size.x);
            let y_bytes: [u8; 4] = std::mem::transmute(window_size.y);
            cam_data.extend_from_slice(&x_bytes);
            cam_data.extend_from_slice(&y_bytes);
        }
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: cam_data.as_slice(),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let cam_sampler_uniform_group =
            Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&device.create_sampler(
                            &wgpu::SamplerDescriptor {
                                address_mode_u: wgpu::AddressMode::Repeat,
                                address_mode_v: wgpu::AddressMode::Repeat,
                                ..Default::default()
                            },
                        )),
                    },
                ],
            }));

        let window_size = unsafe { PLATFORM_DATA.window_size.unwrap() };
        let depth_texture = Texture::new(
            &device,
            window_size.x as u32,
            window_size.y as u32,
            wgpu::TextureFormat::Depth32Float,
        );
        let frame_texture = Texture::new(
            &device,
            window_size.x as u32,
            window_size.y as u32,
            sc_desc.format,
        );
        let frame_texture_backbuffer = Texture::new(
            &device,
            window_size.x as u32,
            window_size.y as u32,
            sc_desc.format,
        );
        let active_decals = vec![];
        Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            size,
            render_pipeline: None,
            render_3D_pipeline_indexed: None,
            decal_buffer,
            decals,
            active_decals,
            decal_counter: 0,
            texture_sampler: Some(decal_sampler),
            bind_group_layout: None,
            bind_group: None,
            layer_textures: None,
            frame: None,
            camera_buffer,
            cam_sampler_uniform_group,
            meshes: vec![],
            game_objects: vec![],
            draw_data: vec![],
            vertex_buffer,
            indexed_vertex_buffer,
            index_buffer,
            layer_shader,
            indexed_vert_shader,
            depth_texture,
            frame_texture,
            frame_texture_backbuffer,
            default_texture,
            default_texture_bind,
            index_count: 0,
            tri_count: 0,
            camera: Camera::default(),
        }
    }

    pub fn get_reference(&self) -> &Self {
        self
    }

    pub fn new_vertex_buffer(&self) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Indexed Vertex Buffer"),
                //This is the maximum number of vertices that can be indexed with a u16
                contents: bytemuck::cast_slice(
                    vec![Vertex::default(); VERT_BUFFER_SIZE].as_slice(),
                ),
                usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            })
    }

    pub fn new_index_buffer(&self) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(vec![0; INDEX_BUFFER_SIZE].as_slice()),
                usage: wgpu::BufferUsage::INDEX | wgpu::BufferUsage::COPY_DST,
            })
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
            entry_point: "vs_main",     // 1.
            buffers: &[Vertex::desc()], // 2.
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
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
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
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
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
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStage::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStage::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Sampler {
                                comparison: false,
                                filtering: false,
                            },
                            count: None,
                        },
                    ],
                });
        let bind_group_layout_layer = self
            .device
            .create_bind_group_layout(&super::layer::DrawData::default_bind_group_layout());
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bind_group_layout, &bind_group_layout_layer],
                push_constant_ranges: &[],
            });
        self.render_3D_pipeline_indexed = Some(self.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &self.indexed_vert_shader,
                    entry_point: "vs_main",
                    buffers: &[Vertex::desc()],
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
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &self.indexed_vert_shader,
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
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            },
                        }),
                        write_mask: wgpu::ColorWrite::ALL,
                    }],
                }),
            },
        ));
    }

    pub fn update_layer_texture_groups(&mut self) {
        //Put the frame_texture as the first thing drawn
        self.layer_textures = Some(
            std::iter::once(
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(
                                &self.frame_texture.texture_bundle.as_ref().unwrap().view,
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(
                                self.texture_sampler.as_ref().unwrap(),
                            ),
                        },
                    ],
                    layout: self
                        .bind_group_layout
                        .as_ref()
                        .expect("No Bind Group Layout"),
                    label: Some("bind group"),
                }),
            )
            .chain(
                self.active_decals
                    .iter()
                    .map(|k| &self.decals[*k as usize])
                    .filter_map(|tex| {
                        if let Some(bundle) = tex.texture_bundle.as_ref() {
                            Some(
                                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                                    entries: &[
                                        wgpu::BindGroupEntry {
                                            binding: 0,
                                            resource: wgpu::BindingResource::TextureView(
                                                &bundle.view,
                                            ),
                                        },
                                        wgpu::BindGroupEntry {
                                            binding: 1,
                                            resource: wgpu::BindingResource::Sampler(
                                                self.texture_sampler.as_ref().unwrap(),
                                            ),
                                        },
                                    ],
                                    layout: self
                                        .bind_group_layout
                                        .as_ref()
                                        .expect("No Bind Group Layout"),
                                    label: Some("bind group"),
                                }),
                            )
                        } else {
                            None
                        }
                    }),
            )
            .collect(),
        );
    }

    pub fn create_shader_module(&self, shader: &str) -> wgpu::ShaderModule {
        self.device
            .create_shader_module(&wgpu::ShaderModuleDescriptor {
                label: Some("Custom Shader"),
                flags: wgpu::ShaderFlags::all(),
                source: wgpu::ShaderSource::Wgsl(shader.into()),
            })
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

    pub fn add_meshes(&mut self, mut meshes: Vec<Mesh>) {
        self.meshes.append(&mut meshes);
        //self.update_vertex_buffer();
    }

    pub fn add_mesh(&mut self, mesh: Mesh) {
        self.meshes.insert(self.meshes.len(), mesh);
        //self.update_vertex_buffer();
    }

    fn initialize_vertex_buffer(&mut self, tris: Vec<u8>) {
        self.vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: tris.as_slice(),
                usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            });
    }

    fn initialize_indexed_vertex_buffer(&mut self, verts: Vec<Vertex>, mut indices: Vec<u32>) {
        for i in indices.as_slice() {
            if i > &(verts.len() as u32) {
                panic!("Index to bad vertex");
            }
        }

        self.indexed_vertex_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Indexed Vertex Buffer"),
                    contents: bytemuck::cast_slice(verts.as_slice()),
                    usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
                });
        self.index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(indices.as_slice()),
                usage: wgpu::BufferUsage::INDEX | wgpu::BufferUsage::COPY_DST,
            });
    }

    fn update_vertex_buffer(&mut self, tris: Vec<u8>) {
        self.queue
            .write_buffer(&self.vertex_buffer, 0_u64, tris.as_slice());
    }

    pub fn add_game_object(&mut self, mut go: GameObject) {
        go.set_buffer_indices(0);
        self.game_objects.insert(self.game_objects.len(), go);

        // let mut verts: Vec<geometry::Vertex> = vec![];
        // let mut indices: Vec<u32> = vec![];
        // for go in &self.game_objects{
        //     let (v, i, _, _) = go.get_vertices_and_indices(verts.len() as u32, indices.len() as u32);
        //     verts.extend(v);
        //     indices.extend(i);
        // }
        // self.initialize_indexed_vertex_buffer(verts, indices);
    }

    pub fn add_game_objects(&mut self, mut gos: Vec<GameObject>) {
        for go in gos.iter_mut() {
            go.set_buffer_indices(0);
        }

        self.game_objects.extend(gos);
    }

    pub fn add_object_textures(&mut self) {
        todo!()
    }

    pub fn get_tri_data(&self) -> Vec<u8> {
        self.game_objects.iter().fold(vec![], |mut acc, go| {
            let tris = go.get_triangles();
            acc.extend_from_slice(bytemuck::cast_slice::<Triangle, u8>(tris.as_slice()));
            acc
            //m.tris.len() * std::mem::size_of::<geometry::Triangle>()
        })
    }
    pub fn get_transformed_tri_data(&self) -> Vec<u8> {
        self.game_objects.iter().fold(vec![], |mut acc, go| {
            let tris = go.get_transformed_triangles();
            acc.extend_from_slice(bytemuck::cast_slice::<Triangle, u8>(tris.as_slice()));
            acc
            //m.tris.len() * std::mem::size_of::<geometry::Triangle>()
        })
    }

    pub fn get_indices_by_layer(&self, mask: Mask) -> Vec<u32> {
        self.game_objects
            .iter()
            .filter_map(|go| {
                if go.in_layer_mask(mask) {
                    Some(go.get_indices())
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }
    pub fn get_objects_by_layer(&self, mask: Mask) -> Vec<&GameObject> {
        self.game_objects
            .iter()
            .filter(|go| go.in_layer_mask(mask))
            .collect()
    }

    ///This is the actual 3D drawing part.
    /// Takes a pipeline, and a range of indices and creates an end to end encoder and processes it.
    pub fn draw<'a>(
        &'a self,
        indices: std::ops::Range<u32>,
        tex_bind: Option<&'a wgpu::BindGroup>,
        vertex_buffer: Option<&'a wgpu::Buffer>,
        index_buffer: Option<&'a wgpu::Buffer>,
        render_pass: &mut wgpu::RenderPass<'a>,
    ) {
        render_pass.set_bind_group(0, self.cam_sampler_uniform_group.as_ref().unwrap(), &[]);

        if let Some(bg) = tex_bind.as_ref() {
            render_pass.set_bind_group(1, bg, &[]);
        } else {
            render_pass.set_bind_group(1, &self.default_texture_bind, &[]);
        }

        if let Some(v_buffer) = vertex_buffer {
            render_pass.set_vertex_buffer(0, v_buffer.slice(..));
        } else {
            render_pass.set_vertex_buffer(0, self.indexed_vertex_buffer.slice(..));
        }

        if let Some(i_buffer) = index_buffer {
            render_pass.set_index_buffer(i_buffer.slice(..), wgpu::IndexFormat::Uint32);
        } else {
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        }
        render_pass.draw_indexed(indices, 0, 0..1);
    }

    pub fn draw_mask(
        &self,
        camera: &Camera,
        mask: Mask,
        target: &Texture,
        clear_color: Option<wgpu::Color>,
        clear_depth: bool,
        pipeline: Option<&wgpu::RenderPipeline>,
    ) {
        if self.meshes.is_empty() && self.game_objects.is_empty() {
            return;
        }

        let frame = self.get_frame().expect("Couldn't get frame");
        let mut cam_data: Vec<u8> = bytemuck::cast_slice(&[camera.mat]).into();
        let window_size = unsafe { PLATFORM_DATA.window_size.as_ref().unwrap() };
        let (x, y) = (window_size.x as f32, window_size.y as f32);
        let x_bytes: [u8; 4] = x.to_ne_bytes();
        let y_bytes: [u8; 4] = y.to_ne_bytes();

        cam_data.extend_from_slice(&x_bytes);
        cam_data.extend_from_slice(&y_bytes);
        self.queue
            .write_buffer(&self.camera_buffer, 0, cam_data.as_slice());
        let pipeline = if let Some(pipeline) = pipeline.as_ref() {
            pipeline
        } else {
            self.render_3D_pipeline_indexed.as_ref().unwrap()
        };
        let color_attachment = if let Some(color) = clear_color {
            [wgpu::RenderPassColorAttachment {
                view: &target.texture_bundle.as_ref().unwrap().view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(color),
                    store: true,
                },
            }]
        } else {
            [wgpu::RenderPassColorAttachment {
                view: &target.texture_bundle.as_ref().unwrap().view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            }]
        };
        let depth_ops = if clear_depth {
            wgpu::LoadOp::Clear(1.0)
        } else {
            wgpu::LoadOp::Load
        };
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &color_attachment,
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.texture_bundle.as_ref().unwrap().view,
                    depth_ops: Some(wgpu::Operations {
                        load: depth_ops,
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            render_pass.set_pipeline(pipeline);
            //Draw all layers that contain the mask
            for layer_mask in self.draw_data.iter().filter(|d| d.mask.contains(mask)) {
                for (range, bg) in &layer_mask.texture_groups {
                    self.draw(
                        range.clone(),
                        bg.as_ref(),
                        Some(&layer_mask.vertex_buffer),
                        Some(&layer_mask.index_buffer),
                        &mut render_pass,
                    );
                }
            }
        }
        self.queue.submit(std::iter::once(encoder.finish()));
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
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            if let Some(render_pipeline) = self.render_pipeline.as_ref() {
                render_pass.set_pipeline(render_pipeline);
            }
            if let Some(textures) = self.layer_textures.as_ref() {
                for tex_group in textures {
                    render_pass.set_bind_group(0, tex_group, &[]);
                    render_pass.set_vertex_buffer(0, self.decal_buffer.slice(..));
                    render_pass.draw(0..6, 0..1);
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
        #[cfg(target_arch = "wasm32")]
        let format = self.sc_desc.format;

        #[cfg(not(target_arch = "wasm32"))]
        let format = wgpu::TextureFormat::Rgba8Unorm;

        let texture = Texture::new(&self.device, width, height, format);

        self.decals.insert(self.decal_counter as usize, texture);
        self.active_decals
            .insert(self.active_decals.len(), self.decal_counter as u32);
        self.decal_counter += 1;
        //return the newly created layer's id
        self.decal_counter - 1
    }

    pub fn delete_texture(mut id: &mut u32) -> u32 {
        0
    }

    pub fn apply_texture(id: u32) {
        //add Layer View TextureViews in renderer.texture_views
    }

    pub fn update_texture(&self, id: u32, spr: &Sprite) {
        let size = wgpu::Extent3d {
            width: spr.width,
            height: spr.height,
            depth_or_array_layers: 1,
        };
        if let Some(bundle) = &self.decals[id as usize].texture_bundle {
            self.queue.write_texture(
                wgpu::ImageCopyTexture {
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    texture: &bundle.texture,
                },
                spr.get_data(),
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: core::num::NonZeroU32::new(4 * spr.width as u32),
                    rows_per_image: core::num::NonZeroU32::new(0),
                },
                size,
            );
        }
    }
    pub fn update_texture_region(
        &self,
        id: u32,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        data: &[Pixel],
    ) {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let origin = wgpu::Origin3d { x, y, z: 0 };
        if let Some(bundle) = &self.decals[id as usize].texture_bundle {
            self.queue.write_texture(
                wgpu::ImageCopyTexture {
                    mip_level: 0,
                    origin,
                    texture: &bundle.texture,
                },
                bytemuck::cast_slice(data),
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: core::num::NonZeroU32::new(4 * width),
                    rows_per_image: core::num::NonZeroU32::new(0),
                },
                size,
            );
        }
    }

    pub fn add_draw_data_with_objects(&mut self, mask: Mask, game_objects: &[&GameObject]) {
        self.draw_data.insert(
            self.draw_data.len(),
            super::layer::DrawData::empty(mask, self).initialize(
                &self.device,
                &self.queue,
                game_objects,
            ),
        );
    }

    pub fn add_draw_data(&mut self, mask: Mask) {
        self.draw_data.insert(
            self.draw_data.len(),
            super::layer::DrawData::empty(mask, self).initialize(
                &self.device,
                &self.queue,
                self.game_objects
                    .iter()
                    .filter(|go| go.layer_mask.contains(mask))
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
        );
    }

    pub fn update_layer_draw_data(&mut self, mask: Mask) {
        for draw_data in &mut self
            .draw_data
            .iter_mut()
            .filter(|dd| dd.mask.contains(mask))
        {
            draw_data.update(
                &self.device,
                &self.queue,
                self.game_objects
                    .iter()
                    .filter(|go| go.layer_mask.contains(mask))
                    .collect::<Vec<_>>()
                    .as_slice(),
            );
        }
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
