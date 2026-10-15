use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
use wgpu::*;
use winit::window::Window;

#[derive(Debug, Clone, Copy)]
pub struct RenderCaps {
    pub gpu_driven: bool,
    pub timestamp_queries: bool,
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct VoxelVertex {
    pub position: [f32; 3],
    pub colour: [f32; 3],
}
pub struct Renderer<'a> {
    surface: Surface<'a>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    caps: RenderCaps,
    pipeline: RenderPipeline,
    vertex: Buffer,
    vertex_count: u32,
    camera: Buffer,
    bind_group: BindGroup,
    depth: TextureView,
}
impl<'a> Renderer<'a> {
    pub async fn new(
        window: &'a Window,
        vertices: &[VoxelVertex],
    ) -> Result<Self, RequestDeviceError> {
        let instance = Instance::default();
        let surface = instance.create_surface(window).expect("surface");
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("no compatible GPU");
        let features = adapter.features();
        let caps = RenderCaps {
            gpu_driven: features.contains(Features::MULTI_DRAW_INDIRECT),
            timestamp_queries: features.contains(Features::TIMESTAMP_QUERY),
        };
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: Some("Ferrite device"),
                required_features: Features::empty(),
                required_limits: Limits::downlevel_defaults(),
                memory_hints: MemoryHints::Performance,
                trace: Trace::Off,
            })
            .await?;
        let size = window.inner_size();
        let config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .unwrap();
        surface.configure(&device, &config);
        let vertex = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("terrain vertices"),
            contents: bytemuck::cast_slice(vertices),
            usage: BufferUsages::VERTEX,
        });
        let matrix = camera_matrix(
            config.width,
            config.height,
            Vec3::new(48.0, 132.0, 72.0),
            Vec3::new(-48.0, -34.0, -72.0),
        );
        let camera = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("camera"),
            contents: bytemuck::cast_slice(matrix.as_ref()),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("camera layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("camera group"),
            layout: &layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            }],
        });
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("voxel shader"),
            source: ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("voxel layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("voxel pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[VertexBufferLayout {
                    array_stride: std::mem::size_of::<VoxelVertex>() as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &vertex_attr_array![0=>Float32x3,1=>Float32x3],
                }],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                cull_mode: Some(Face::Back),
                front_face: FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let depth = depth_view(&device, config.width, config.height);
        Ok(Self {
            surface,
            device,
            queue,
            config,
            caps,
            pipeline,
            vertex,
            vertex_count: vertices.len() as u32,
            camera,
            bind_group,
            depth,
        })
    }
    pub fn caps(&self) -> RenderCaps {
        self.caps
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.depth = depth_view(&self.device, width, height);
            let matrix = camera_matrix(
                width,
                height,
                Vec3::new(48.0, 132.0, 72.0),
                Vec3::new(-48.0, -34.0, -72.0),
            );
            self.queue
                .write_buffer(&self.camera, 0, bytemuck::cast_slice(matrix.as_ref()));
        }
    }
    pub fn set_camera(&self, eye: [f32; 3], forward: [f32; 3]) {
        let matrix = camera_matrix(
            self.config.width,
            self.config.height,
            Vec3::from_array(eye),
            Vec3::from_array(forward),
        );
        self.queue
            .write_buffer(&self.camera, 0, bytemuck::cast_slice(matrix.as_ref()));
    }
    pub fn render(&mut self) -> Result<(), SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("terrain"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.12,
                            g: 0.19,
                            b: 0.28,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex.slice(..));
            pass.draw(0..self.vertex_count, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        frame.present();
        Ok(())
    }
}
fn depth_view(device: &Device, width: u32, height: u32) -> TextureView {
    device
        .create_texture(&TextureDescriptor {
            label: Some("reverse-z depth"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&TextureViewDescriptor::default())
}
fn camera_matrix(width: u32, height: u32, eye: Vec3, forward: Vec3) -> Mat4 {
    Mat4::perspective_infinite_reverse_rh(65_f32.to_radians(), width as f32 / height as f32, 0.1)
        * Mat4::look_at_rh(eye, eye + forward, Vec3::Y)
}
const SHADER: &str = r#"struct Camera{matrix:mat4x4<f32>}@group(0)@binding(0)var<uniform>camera:Camera;struct In{@location(0)position:vec3<f32>,@location(1)colour:vec3<f32>}struct Out{@builtin(position)position:vec4<f32>,@location(0)colour:vec3<f32>}@vertex fn vs(i:In)->Out{var o:Out;o.position=camera.matrix*vec4<f32>(i.position,1.0);o.colour=i.colour;return o;}@fragment fn fs(i:Out)->@location(0)vec4<f32>{return vec4<f32>(i.colour,1.0);}"#;
