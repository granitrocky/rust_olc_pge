use super::{
    decal::DecalInstance,
    game_object::GameObject,
    geometry::Vertex,
    pixel::Pixel,
    renderer::Renderer,
    sprite::{Sprite, SpriteMode},
    util::Vf2d,
    olc::OlcData,
};

use bitflags::bitflags;

type Func<D> = Box<dyn Fn(&LayerDesc<D>, &Renderer, &mut D, &mut wgpu::CommandEncoder)>;

pub struct LayerFunc<D: OlcData + 'static> {
    func: Func<D>,
}

impl<D: OlcData + 'static> LayerFunc<D> {
    pub fn empty() -> Self {
        Self {
            func: Box::new(|_, _, _, _| {}),
        }
    }

    pub fn execute(
        &self,
        layer: &LayerDesc<D>,
        renderer: &Renderer,
        game_data: &mut D,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        (self.func)(layer, renderer, game_data, encoder);
    }

    pub fn from(func: Func<D>) -> Self {
        Self { func }
    }

    pub fn default() -> Self {
        Self::from(Box::new(default_layer_func))
    }
}

pub struct LayerDesc<D: OlcData + 'static> {
    pub id: u32,
    pub shown: bool,
    pub layer_info: LayerInfo<D>,
    pub vec_decal_instance: Vec<DecalInstance>,
}

#[derive(Default)]
pub struct Image {
    pub sprite: Sprite,
    pub update: bool,
    pub offset: Vf2d,
    pub scale: Vf2d,
    pub tint: Pixel,
}

pub struct Render<D: OlcData + 'static> {
    pub mask: u32,
    pub pipeline_bundle: Option<PipelineBundle<D>>,
}

pub enum LayerInfo<D: OlcData + 'static> {
    Image(Image),
    Render(Render<D>),
}

pub enum LayerType {
    Image,
    Render,
}

pub trait LayerMask {
    type BitFlags;

    fn in_layer_mask(&self, mask: Self::BitFlags) -> bool;
    fn set_layer_mask(&mut self, mask: Self::BitFlags);
    fn add_layer(&mut self, mask: Self::BitFlags);
    fn remove_layer(&mut self, mask: Self::BitFlags);
    fn reset_mask(&mut self);
}

bitflags! {
    pub struct Mask: u64{
        const D3     = 0b00000001;
        const GUI    = 0b00000010;
        const LAYER3 = 0b00000100;
        const LAYER4 = 0b00001000;
        const LAYER5 = 0b00010000;
        const LAYER6 = 0b00100000;
        const LAYER7 = 0b01000000;
        const LAYER8 = 0b10000000;
    }
}

//Usage: get by mask, fill the index_buffer and pipeline_data::buffer with indices and vertices
//   Put all the mesh textures in the pipeline_data::BindGroup
//   The pipeline_data::BindGroupLayout should have one SAMPLER and one TEXTUREARRAY2D
//   If we want a new shader, we can supply it. Otherwise, use the defauly
pub struct DrawData {
    pub mask: Mask,
    pub index_buffer: wgpu::Buffer,
    pub vertex_buffer: wgpu::Buffer,
    pub texture_groups: Vec<(std::ops::Range<u32>, Option<wgpu::BindGroup>)>,
}

pub struct PipelineData {
    pub pipeline: wgpu::RenderPipeline,
    pub pipeline_layout: wgpu::PipelineLayout,
    pub buffer: wgpu::Buffer,
    pub bind_groups: Vec<wgpu::BindGroup>,
    pub bind_group_layouts: Vec<wgpu::BindGroupLayout>,
    pub shader: wgpu::ShaderModule,
}
pub struct PipelineBundle<D: OlcData + 'static> {
    pub func: LayerFunc<D>,
    pub data: PipelineData,
}

pub static EMPTY_IMAGE: Image = default_image();

const fn default_image() -> Image {
    Image {
        sprite: Sprite {
            col_data: Vec::new(),
            height: 0,
            width: 0,
            mode_sample: SpriteMode::Normal,
        },
        update: false,
        offset: Vf2d { x: 0.0, y: 0.0 },
        scale: Vf2d { x: 0.0, y: 0.0 },
        tint: Pixel::BLANK,
    }
}

impl<D: OlcData + 'static> LayerDesc<D> {
    pub fn empty(layer_type: LayerType) -> Self {
        match layer_type {
            LayerType::Image => LayerDesc {
                id: 0,
                shown: false,
                layer_info: LayerInfo::Image(Image {
                    offset: Default::default(),
                    scale: Default::default(),
                    tint: Pixel::rgb(0, 0, 0),
                    sprite: Sprite::new(0, 0),
                    update: false,
                }),
                vec_decal_instance: vec![],
            },
            LayerType::Render => LayerDesc {
                id: 0,
                shown: false,
                layer_info: LayerInfo::Render(Render {
                    pipeline_bundle: None,
                    mask: 0xFFFFFFFF,
                }),
                vec_decal_instance: vec![],
            },
        }
    }

    pub fn new(layer_info: LayerInfo<D>) -> Self {
        LayerDesc {
            id: 0,
            shown: false,
            layer_info,
            vec_decal_instance: vec![],
        }
    }

    pub fn as_render_layer(&self) -> Option<&Render<D>> {
        if let LayerInfo::Render(render) = &self.layer_info {
            Some(render)
        } else {
            None
        }
    }

    pub fn as_render_layer_mut(&mut self) -> Option<&mut Render<D>> {
        if let LayerInfo::Render(render) = &mut self.layer_info {
            Some(render)
        } else {
            None
        }
    }

    pub fn setup_default_pipeline_data(&mut self, renderer: &Renderer) {
        if let LayerInfo::Render(render_info) = &mut self.layer_info {
            render_info.pipeline_bundle = Some(PipelineBundle::default(renderer))
        }
    }
}

impl<D: OlcData + 'static> PipelineBundle<D> {
    pub fn create(func: LayerFunc<D>, data: PipelineData) -> Self {
        Self { func, data }
    }

    pub fn default(renderer: &Renderer) -> Self {
        PipelineBundle {
            func: LayerFunc::default(),
            data: PipelineData::default(renderer),
        }
    }
}

//Probably need a better name for this. DrawData?
impl DrawData {
    /*
    New plan.

    Go through each mesh and
    1) Fill the Vertex Buffer
    2) create a DRAW_CALL style object with:
       a) Range of u32
       b) BindGroup that holds the Texture
     */
    pub fn initialize(
        mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        game_objects: &[&GameObject],
    ) -> Self {
        self.update(device, queue, game_objects);
        self
    }

    pub fn update(&mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        game_objects: &[&GameObject],){
        let mut vertices: Vec<Vertex> = vec![];
        let mut indices: Vec<u32> = vec![];
        let (mut index_count, mut vertex_count) = (0, 0);
        for (_i, go) in game_objects.iter().enumerate() {
            let (verts, inds, vc, _) = go.get_vertices_and_indices(vertex_count, index_count);
            for mesh in &go.meshes {
                let tex = mesh.get_texture().as_ref().and_then(|tex| {
                    tex.texture_bundle.as_ref().map(|bundle| {
                        device.create_bind_group(&wgpu::BindGroupDescriptor {
                            entries: &[wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&bundle.view),
                            }],
                            label: None,
                            layout: &device
                                .create_bind_group_layout(&DrawData::default_bind_group_layout()),
                        })
                    })
                });

                let mesh_index_count = mesh.buffer_indices.len() as u32;
                let i_range = index_count..index_count + mesh_index_count;
                self.texture_groups
                    .insert(self.texture_groups.len(), (i_range, tex));
                index_count += mesh_index_count;
            }

            vertices.extend(verts);
            indices.extend(inds);
            vertex_count = vc;
        }
        println!("Verts: {}, Indices: {}", vertices.len(), indices.len());
        //Fill index_buffer
        queue.write_buffer(
            &self.index_buffer,
            0,
            bytemuck::cast_slice(indices.as_slice()),
        );
        //Fill vertex::buffer
        queue.write_buffer(
            &self.vertex_buffer,
            0,
            bytemuck::cast_slice(vertices.as_slice()),
        );
    }

    //This would be a good time to send the transforms so they are processed on the GPU instead
    pub fn default_bind_group_layout<'a>() -> wgpu::BindGroupLayoutDescriptor<'a> {
        wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
            label: None,
        }
    }

    pub fn empty(mask: Mask, renderer: &Renderer) -> Self {
        Self {
            mask,
            index_buffer: renderer.new_index_buffer(),
            vertex_buffer: renderer.new_vertex_buffer(),
            texture_groups: vec![],
        }
    }
}

impl PipelineData {
    pub fn default(renderer: &Renderer) -> Self {
        let shader = renderer
            .device
            .create_shader_module(&wgpu::ShaderModuleDescriptor {
                label: Some("default_shader"),
                flags: wgpu::ShaderFlags::all(),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/default_postprocess.wgsl").into()),
            });
        let sc_desc = &[wgpu::ColorTargetState {
            format: renderer.sc_desc.format,
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
        let bind_group_layouts = vec![
            renderer
                .device
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
                }),
            renderer
                .device
                .create_bind_group_layout(&DrawData::default_bind_group_layout()),
        ];
        let bind_groups = vec![
            renderer
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: renderer.camera_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(
                                renderer.texture_sampler.as_ref().unwrap(),
                            ),
                        },
                    ],
                    layout: &bind_group_layouts[0],
                    label: Some("blur group"),
                }),
            renderer
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &renderer
                                .frame_texture_backbuffer
                                .texture_bundle
                                .as_ref()
                                .unwrap()
                                .view,
                        ),
                    }],
                    layout: &bind_group_layouts[1],
                    label: Some("Backbuffer Texture Bind"),
                }),
        ];

        let pipeline_layout =
            renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &bind_group_layouts.iter().collect::<Vec<_>>(),
                    push_constant_ranges: &[],
                });
        let render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",     // 1.
                buffers: &[Vertex::desc()], // 2.
            },
            fragment: Some(wgpu::FragmentState {
                // 3.
                module: &shader,
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
        let pipeline = renderer
            .device
            .create_render_pipeline(&render_pipeline_descriptor);

        use wgpu::util::DeviceExt;
        let buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                usage: wgpu::BufferUsage::UNIFORM,
                contents: bytemuck::cast_slice(&[0; 1]),
            });
        Self {
            pipeline,
            pipeline_layout,
            bind_groups,
            bind_group_layouts,
            shader,
            buffer,
        }
    }

    pub fn set_bind_group_layouts(&mut self, layout: Vec<wgpu::BindGroupLayout>) {
        self.bind_group_layouts = layout;
    }

    pub fn add_bind_group_layout(&mut self, layout: wgpu::BindGroupLayout) {
        self.bind_group_layouts
            .insert(self.bind_group_layouts.len(), layout);
    }
    pub fn set_bind_groups(&mut self, groups: Vec<wgpu::BindGroup>) {
        self.bind_groups = groups;
    }

    pub fn add_bind_group(&mut self, group: wgpu::BindGroup) {
        self.bind_groups.insert(self.bind_groups.len(), group);
    }

    pub fn update_shader(&mut self, shader: wgpu::ShaderModule) {
        self.shader = shader;
    }

    pub fn rebuild_pipeline(&mut self, renderer: &Renderer, use_depth: bool) {
        self.pipeline_layout =
            renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &self.bind_group_layouts.iter().collect::<Vec<_>>(),
                    push_constant_ranges: &[],
                });
        let color_target = &[wgpu::ColorTargetState {
            format: renderer.sc_desc.format,
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
        let depth_stencil = if use_depth {
            Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
        } else {
            None
        };
        let render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&self.pipeline_layout),
            vertex: wgpu::VertexState {
                module: &self.shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                // 3.
                module: &self.shader,
                entry_point: "fs_main",
                targets: color_target,
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                clamp_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil,
            multisample: wgpu::MultisampleState::default(),
        };
        self.pipeline = renderer
            .device
            .create_render_pipeline(&render_pipeline_descriptor);
    }
}

pub fn default_layer_func<D: OlcData>(
    layer: &LayerDesc<D>,
    renderer: &Renderer,
    _game_data: &mut D,
    encoder: &mut wgpu::CommandEncoder,
) {
    if let LayerInfo::Render(render_info) = &layer.layer_info {
        let _frame = renderer.get_frame().expect("Can't get Frame");

        {
            if let Some(pipeline_data) = &render_info.pipeline_bundle {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Layer Pass"),
                    color_attachments: &[wgpu::RenderPassColorAttachment {
                        view: &renderer
                            .frame_texture_backbuffer
                            .texture_bundle
                            .as_ref()
                            .unwrap()
                            .view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
                });
                render_pass.set_pipeline(&pipeline_data.data.pipeline);
                for (i, bg) in pipeline_data.data.bind_groups.iter().enumerate() {
                    render_pass.set_bind_group(i as u32, bg, &[]);
                }
                render_pass.set_vertex_buffer(0, renderer.decal_buffer.slice(..));
                render_pass.draw(0..6, 0..1);
            }
        }
    }
}
