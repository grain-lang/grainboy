#![allow(unused)]

use std::ops::{Add, Div, Mul, Sub};

#[derive(Debug)]
pub struct Renderer<'a> {
    pub globals: Globals,
    pub frame_dur: instant::Duration,
    pub first_tick: instant::Instant,
    u_globals: UniformBuffer<Globals>,
    v_surface: VertexBuffer<'a>,
    v_canvas: VertexBuffer<'a>,
    canvas: Canvas,
    spritesheet: Spritesheet,
    pipeline: wgpu::RenderPipeline,
    clear_color: wgpu::Color,
}
impl<'a> Renderer<'a> {
    pub fn new(gpu: &GPUContext) -> Self {
        let shader_module = gpu
            .device
            .create_shader_module(wgpu::include_wgsl!("main.wgsl"));
        let v_canvas = VertexBuffer::new(&gpu.device);
        let canvas = Canvas::new(&gpu.device, &gpu.queue, &gpu.config.format, 256, 144);
        let mut v_surface = VertexBuffer::new(&gpu.device);
        v_surface.write(&gpu.queue, 0, &canvas.vertex_bytes.clone());
        let globals = Globals::new([
            canvas.texture.width() as f32,
            canvas.texture.height() as f32,
        ]);
        let u_globals = UniformBuffer::new(&gpu.device, globals);
        let spritesheet = Spritesheet::new(&gpu.device, &gpu.queue);
        let pipeline = Self::create_render_pipeline(
            gpu,
            &[&u_globals.layout, &spritesheet.layout],
            &shader_module,
            v_canvas.layouts,
        );
        Self {
            globals,
            frame_dur: instant::Duration::from_secs(1).div(60),
            first_tick: instant::Instant::now(),
            u_globals,
            v_surface,
            v_canvas,
            canvas,
            spritesheet,
            pipeline,
            clear_color: wgpu::Color {
                r: 0.,
                g: 0.,
                b: 0.,
                a: 1.0,
            },
        }
    }
    fn create_render_pipeline(
        gpu: &GPUContext,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
        shader_module: &wgpu::ShaderModule,
        buffers: &[wgpu::VertexBufferLayout],
    ) -> wgpu::RenderPipeline {
        gpu.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Grainboy Render Pipeline"),
                layout: Some(
                    &gpu.device
                        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                            label: Some("Grainboy Render Pipeline Layout"),
                            bind_group_layouts,
                            push_constant_ranges: &[],
                        }),
                ),
                vertex: wgpu::VertexState {
                    module: shader_module,
                    entry_point: "vs_main",
                    buffers,
                },
                fragment: Some(wgpu::FragmentState {
                    module: shader_module,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: gpu.config.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                multisample: wgpu::MultisampleState::default(),
                depth_stencil: None,
                multiview: None,
            })
    }
    pub fn set_clear_color(&mut self, color: wgpu::Color) {
        self.clear_color = color;
    }
    pub fn write_vertexes(&mut self, gpu: &GPUContext, data: &[u8]) {
        self.v_canvas.write(&gpu.queue, 0, data);
    }
    pub fn write_uniform(&self, gpu: &GPUContext, data: &[u8]) {
        self.u_globals.write(&gpu.queue, 0, data);
    }
    pub fn render(&mut self, gpu: &GPUContext) -> Result<instant::Instant, wgpu::SurfaceError> {
        let now = instant::Instant::now();
        let delta = now.sub(self.first_tick);
        let tick = f32::floor(delta.as_secs_f32() / self.frame_dur.as_secs_f32()) as u32;
        if tick > self.globals.tick {
            self.globals.tick = tick;
            self.write_uniform(gpu, &bytemuck::cast_slice(&[self.globals]));
            if self.v_canvas.count > 0 {
                self.render_canvas(gpu);
                self.render_surface(gpu)?;
            }
        }
        let tick_dur = self.frame_dur.mul(tick + 1);
        let next = self.first_tick.add(tick_dur);
        return Ok(next);
    }
    fn render_canvas(&mut self, gpu: &GPUContext) {
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Grainboy Canvas Render Encoder"),
            });
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Grainboy Canvas Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.canvas.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.clear_color),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });
        let w = self.canvas.texture.width();
        let h = self.canvas.texture.height();
        render_pass.set_viewport(0., 0., w as f32, h as f32, 0.0, 1.0);
        render_pass.set_scissor_rect(0, 0, w, h);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.u_globals.bind_group, &[]);
        render_pass.set_bind_group(1, &self.spritesheet.bind_group, &[]);
        let vxs = &self.v_canvas;
        let num_quads = vxs.count as u64;
        render_pass.set_vertex_buffer(0, {
            let size = std::mem::size_of::<QuadVertex>() as u64;
            let count = num_quads;
            let limit = size * count;
            let bounds = 0..limit;
            vxs.quads.slice(bounds)
        });
        render_pass.set_vertex_buffer(1, {
            let size = std::mem::size_of::<IndexVertex>() as u64;
            let count = num_quads * 6;
            let limit = size * count;
            let bounds = 0..limit;
            vxs.indices.slice(bounds)
        });
        let num_quads = num_quads as u32;
        render_pass.draw(0..(num_quads * 6), 0..num_quads);
        drop(render_pass);
        let cmd_buf = encoder.finish();
        gpu.queue.submit(std::iter::once(cmd_buf));
    }
    fn render_surface(&mut self, gpu: &GPUContext) -> Result<(), wgpu::SurfaceError> {
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Grainboy Surface Render Encoder"),
            });
        let frame = gpu.surface.get_current_texture()?;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            ..Default::default()
        });
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Grainboy Surface Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 32. / 255. / 10.,
                        g: 33. / 255. / 10.,
                        b: 36. / 255. / 10.,
                        a: 1.0,
                    }),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });
        let ww = gpu.config.width as f32;
        let wh = gpu.config.height as f32;
        let cw = self.canvas.texture.width() as f32;
        let ch = self.canvas.texture.height() as f32;
        let aspect_ratio = cw / ch;
        let (vw, vh) = if ww <= wh * aspect_ratio {
            let vw = ww;
            let vh = vw / aspect_ratio;
            (vw, vh)
        } else {
            let vh = wh;
            let vw = vh * aspect_ratio;
            (vw, vh)
        };
        let vx = (ww - vw) / 2.;
        let vy = (wh - vh) / 2.;
        render_pass.set_viewport(vx, vy, vw, vh, 1., 1.);
        render_pass.set_scissor_rect(vx as u32, vy as u32, vw as u32, vh as u32);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.u_globals.bind_group, &[]);
        render_pass.set_bind_group(1, &self.canvas.bind_group, &[]);
        let vxs = &self.v_surface;
        let num_quads = vxs.count as u64;
        render_pass.set_vertex_buffer(0, {
            let size = std::mem::size_of::<QuadVertex>() as u64;
            let count = num_quads;
            let limit = size * count;
            let bounds = 0..limit;
            vxs.quads.slice(bounds)
        });
        render_pass.set_vertex_buffer(1, {
            let size = std::mem::size_of::<IndexVertex>() as u64;
            let count = num_quads * 6;
            let limit = size * count;
            let bounds = 0..limit;
            vxs.indices.slice(bounds)
        });
        let num_quads = num_quads as u32;
        render_pass.draw(0..(num_quads * 6), 0..num_quads);
        drop(render_pass);
        let cmd_buf = encoder.finish();
        gpu.queue.submit(std::iter::once(cmd_buf));
        frame.present();
        Ok(())
    }
}

#[derive(Debug)]
pub struct GPUContext {
    pub window: winit::window::Window,
    pub surface: wgpu::Surface,
    pub capabilities: wgpu::SurfaceCapabilities,
    pub config: wgpu::SurfaceConfiguration,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}
impl GPUContext {
    pub async fn new(window: winit::window::Window) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: wgpu::Dx12Compiler::default(),
        });
        let surface = unsafe {
            instance
                .create_surface(&window)
                .expect("Couldn't create surface")
        };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Couldn't find an adapter with requested options");
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("WGPU Device"),
                    features: wgpu::Features::empty(),
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                },
                None,
            )
            .await
            .expect("Couldn't create");
        let capabilities = surface.get_capabilities(&adapter);
        let default_format = *capabilities
            .formats
            .get(0)
            .expect("No surface formats available");
        let surface_format = capabilities
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(default_format);
        let present_mode = *capabilities
            .present_modes
            .get(0)
            .expect("No surface present modes available");
        let default_alpha_mode = *capabilities
            .alpha_modes
            .get(0)
            .expect("No surface alpha modes available");
        let alpha_mode = capabilities
            .alpha_modes
            .iter()
            .copied()
            // The compositor will multiply the non-alpha channels of the texture by the alpha channel during compositing
            .find(|a| *a == wgpu::CompositeAlphaMode::PostMultiplied)
            .unwrap_or(default_alpha_mode);
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            width: size.width,
            height: size.height,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            present_mode,
            alpha_mode,
            view_formats: vec![],
        };
        surface.configure(&device, &config);
        Self {
            window,
            surface,
            capabilities,
            config,
            adapter,
            device,
            queue,
        }
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Globals {
    pub viewport: [f32; 2],
    pub scale: f32,
    pub tick: u32,
    // pub transition: [f32; 4], // prop: u8 | dur: u8 | timing_fn: u8
}
impl Globals {
    pub fn new(viewport: [f32; 2]) -> Self {
        Self {
            viewport,
            scale: 1.,
            tick: 0,
        }
    }
}
#[derive(Debug)]
pub struct UniformBuffer<T> {
    pub uniform: T,
    pub buffer: wgpu::Buffer,
    pub layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}
impl<T: bytemuck::Pod + bytemuck::Zeroable> UniformBuffer<T> {
    pub fn new(device: &wgpu::Device, uniform: T) -> Self {
        use wgpu::util::DeviceExt;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grainboy Globals Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Grainboy Globals BindGroupLayout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    // min_binding_size: None,
                    min_binding_size: wgpu::BufferSize::new(
                        // Must have a size that is a multiple of 16 bytes
                        std::mem::size_of::<Globals>() as u64,
                    ),
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grainboy Uniform BindGroup"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        Self {
            uniform,
            buffer,
            layout,
            bind_group,
        }
    }
    /// Writes data into the buffer
    pub fn write(&self, queue: &wgpu::Queue, offset: wgpu::BufferAddress, data: &[u8]) {
        queue.write_buffer(&self.buffer, offset, data)
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct IndexVertex {
    pub index: u32,
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadVertex {
    pub rect: [f32; 4],            // xywh
    pub fill: u32,                 // rgba
    pub tex_rect: [f32; 4],        // xywh
    pub tex_fill: u32,             // texture mix amount
    pub rotation_base: f32,        // base rotation radians
    pub rotation_rate: f32,        // additional rotation radians per tick
    pub rotation_origin: [f32; 2], // rotation origin (relative to top-left of quad)
    pub border_radius: [u32; 2],
    pub border_size: u32,
    pub border_color: [u32; 4],
}
impl QuadVertex {
    pub const ATTRIBUTE_ARRAY: [wgpu::VertexAttribute; 10] = wgpu::vertex_attr_array![
        1 => Float32x4,  // rect
        2 => Uint32,     // fill
        3 => Float32x4,  // tex_rect
        4 => Uint32,     // tex_fill
        5 => Float32,    // rotation_base
        6 => Float32,    // rotation_rate (TODO: move to animation uniform)
        7 => Float32x2,  // rotation_origin
        8 => Uint32x2,   // border_radius
        9 => Uint32,     // border_size (top, right, bottom left)
        10 => Uint32x4,  // border_color (top, right, bottom left)
    ];
    pub const fn new(rect: [f32; 4]) -> Self {
        Self {
            rect,
            fill: 0,
            tex_rect: [0.; 4],
            tex_fill: 0,
            rotation_base: 0.,
            rotation_rate: 0.,
            rotation_origin: [0.; 2],
            border_radius: [0; 2],
            border_size: 0,
            border_color: [0; 4],
        }
    }
    pub const fn tex_rect(&self, tex_rect: [f32; 4]) -> Self {
        Self { tex_rect, ..*self }
    }
    pub const fn border_radius(&self, border_radius: [(u32, u32); 4]) -> Self {
        let [tl, tr, br, bl] = border_radius;
        let xs = (bl.0 << 24) | (br.0 << 16) | (tr.0 << 8) | tl.0;
        let ys = (bl.1 << 24) | (br.1 << 16) | (tr.1 << 8) | tl.1;
        Self {
            border_radius: [xs, ys],
            ..*self
        }
    }
}

#[derive(Debug)]
pub struct VertexBuffer<'a> {
    pub quads: wgpu::Buffer,
    pub indices: wgpu::Buffer,
    pub count: usize,
    pub layouts: &'a [wgpu::VertexBufferLayout<'a>],
}
impl<'a> VertexBuffer<'a> {
    pub fn new(device: &wgpu::Device) -> Self {
        use wgpu::util::DeviceExt;
        // Gotta be careful about device.limits.maxBufferSize
        const MAX_QUADS: usize = 1_000_000;
        let quads = {
            let contents = &[0; MAX_QUADS * std::mem::size_of::<QuadVertex>()];
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Grainboy Quad Vertex Buffer"),
                contents,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            })
        };
        let indices = {
            let contents: Vec<IndexVertex> = (0..MAX_QUADS * 6)
                .enumerate()
                .map(|(i, _)| IndexVertex {
                    index: i as u32 % 6,
                })
                .collect();
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Grainboy Index Vertex Buffer"),
                contents: bytemuck::cast_slice(&contents),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            })
        };
        Self {
            quads,
            indices,
            count: 0,
            layouts: &[
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &QuadVertex::ATTRIBUTE_ARRAY,
                },
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<IndexVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Uint32,
                        offset: std::mem::size_of::<QuadVertex>() as u64,
                        shader_location: 0,
                    }],
                },
            ],
        }
    }
    pub fn write(&mut self, queue: &wgpu::Queue, offset: wgpu::BufferAddress, data: &[u8]) {
        queue.write_buffer(&self.quads, offset, data);
        self.count = data.len() / std::mem::size_of::<QuadVertex>();
    }
}

#[derive(Debug)]
pub struct Canvas {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub vertex_bytes: Vec<u8>,
}
impl Canvas {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: &wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Grainboy Canvas Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: *format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Grainboy Canvas Texture BindGroupLayout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            ..Default::default()
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grainboy Canvas Texture BindGroup"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&device.create_sampler(
                        &wgpu::SamplerDescriptor {
                            address_mode_u: wgpu::AddressMode::ClampToEdge,
                            address_mode_v: wgpu::AddressMode::ClampToEdge,
                            address_mode_w: wgpu::AddressMode::ClampToEdge,
                            mag_filter: wgpu::FilterMode::Nearest,
                            min_filter: wgpu::FilterMode::Nearest,
                            mipmap_filter: wgpu::FilterMode::Nearest,
                            ..Default::default()
                        },
                    )),
                },
            ],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &vec![0; (size.width * size.height * 4) as usize],
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(
                    std::num::NonZeroU32::new(4 * size.width)
                        .expect("Image dimensions are invalid non-zero uint32")
                        .into(),
                ),
                rows_per_image: Some(
                    std::num::NonZeroU32::new(size.height)
                        .expect("Image dimensions are invalid non-zero uint32")
                        .into(),
                ),
            },
            size,
        );
        let tex_size = [0., 0., size.width as f32, size.height as f32];
        let quad = QuadVertex::new(tex_size).tex_rect(tex_size);
        Self {
            texture,
            view,
            layout,
            bind_group,
            vertex_bytes: bytemuck::cast_slice(&[quad]).to_vec(),
        }
    }
}

#[derive(Debug)]
pub struct Spritesheet {
    pub texture: wgpu::Texture,
    pub layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}
impl Spritesheet {
    const BYTES: &'static [u8] = include_bytes!("spritesheet.png");
    const EXTENT: wgpu::Extent3d = wgpu::Extent3d {
        width: 256,
        height: 1280,
        depth_or_array_layers: 1,
    };
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let image = image::load_from_memory(Self::BYTES).unwrap();
        let rgba = image.to_rgba8();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Grainboy Spritesheet Texture"),
            size: Self::EXTENT,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Grainboy Spritesheet Texture BindGroupLayout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grainboy Spritesheet Texture BindGroup"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&device.create_sampler(
                        &wgpu::SamplerDescriptor {
                            address_mode_u: wgpu::AddressMode::ClampToEdge,
                            address_mode_v: wgpu::AddressMode::ClampToEdge,
                            address_mode_w: wgpu::AddressMode::ClampToEdge,
                            mag_filter: wgpu::FilterMode::Nearest,
                            min_filter: wgpu::FilterMode::Nearest,
                            mipmap_filter: wgpu::FilterMode::Nearest,
                            ..Default::default()
                        },
                    )),
                },
            ],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(
                    std::num::NonZeroU32::new(4 * Self::EXTENT.width)
                        .expect("Image dimensions are invalid non-zero uint32")
                        .into(),
                ),
                rows_per_image: Some(
                    std::num::NonZeroU32::new(Self::EXTENT.height)
                        .expect("Image dimensions are invalid non-zero uint32")
                        .into(),
                ),
            },
            Self::EXTENT,
        );
        Self {
            texture,
            layout,
            bind_group,
        }
    }
}
