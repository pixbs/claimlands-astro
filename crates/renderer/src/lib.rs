//! GPU resources owned by one application surface; no authoritative game state.
#![cfg(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))]
use claimlands_visuals::{PlanetAssets, Vertex};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use wgpu::util::DeviceExt;

/// Requested graphics backend, with automatic browser fallback by default.
#[derive(Clone, Copy, Debug, Default)]
pub enum Backend {
    /// Prefer the best available backend.
    #[default]
    Auto,
    /// Force WebGL2 for compatibility testing.
    WebGl,
    /// Force WebGPU for compatibility testing.
    WebGpu,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    mvp: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    selection: [f32; 4],
}

/// A renderer whose resources are dropped with the associated window lifecycle.
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    instance: wgpu::Instance,
    window: Arc<winit::window::Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    blit_pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    bindings: wgpu::BindGroup,
    vertex: wgpu::Buffer,
    index: wgpu::Buffer,
    index_count: u32,
    low_color: wgpu::TextureView,
    low_depth: wgpu::TextureView,
    blit_bindings: wgpu::BindGroup,
    ui: egui_wgpu::Renderer,
    backend: String,
    device_lost: Arc<AtomicBool>,
    failure: Arc<Mutex<Option<String>>>,
}

impl Renderer {
    /// Create GPU resources. The window owns the underlying surface handle.
    pub async fn new(
        window: Arc<winit::window::Window>,
        assets: &PlanetAssets,
        backend: Backend,
    ) -> Result<Self, String> {
        let backends = match backend {
            Backend::Auto => wgpu::Backends::all(),
            Backend::WebGl => wgpu::Backends::GL,
            Backend::WebGpu => wgpu::Backends::BROWSER_WEBGPU,
        };
        let descriptor = wgpu::InstanceDescriptor {
            backends,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        };
        let instance = match backend {
            Backend::Auto => wgpu::util::new_instance_with_webgpu_detection(descriptor).await,
            _ => wgpu::Instance::new(descriptor),
        };
        let size = window.inner_size();
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| e.to_string())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await
            .map_err(|e| e.to_string())?;
        let backend = format!("{:?}", adapter.get_info().backend);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Claim Lands device"),
                required_features: wgpu::Features::empty(),
                // Keep feature limits portable, but permit the adapter's actual
                // display resolution (phone surfaces commonly exceed 2048px).
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                ..Default::default()
            })
            .await
            .map_err(|e| e.to_string())?;
        let device_lost = Arc::new(AtomicBool::new(false));
        let lost_flag = device_lost.clone();
        device.set_device_lost_callback(move |_, _| lost_flag.store(true, Ordering::Release));
        let failure = Arc::new(Mutex::new(None));
        let device_failure = failure.clone();
        device.on_uncaptured_error(Arc::new(move |error| {
            let mut failure = device_failure.lock().unwrap_or_else(|e| e.into_inner());
            // Keep the first diagnostic; repeated validation failures must not grow memory.
            if failure.is_none() {
                *failure = Some(format!("graphics device error: {error}"));
            }
        }));
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            color_space: wgpu::SurfaceColorSpace::Auto,
            view_formats: vec![],
        };
        surface.configure(&device, &config);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("planet"),
            source: wgpu::ShaderSource::Wgsl(include_str!("planet.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("planet"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3,2=>Float32x2],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment_main"),
                compilation_options: Default::default(),
                targets: &[Some(format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera"),
            size: 144,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bindings = Self::assets_bindings(&device, &queue, &pipeline, &uniform, assets);
        let (vertex, index, index_count) = Self::buffers(&device, assets);
        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pixel upscale"),
            source: wgpu::ShaderSource::Wgsl(include_str!("blit.wgsl").into()),
        });
        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("nearest upscale"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &blit_shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(format.into())],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let (low_color, low_depth, blit_bindings) = Self::targets(&device, &config, &blit_pipeline);
        let ui = egui_wgpu::Renderer::new(&device, format, egui_wgpu::RendererOptions::default());
        Ok(Self {
            surface,
            instance,
            window,
            device,
            queue,
            config,
            pipeline,
            blit_pipeline,
            uniform,
            bindings,
            vertex,
            index,
            index_count,
            low_color,
            low_depth,
            blit_bindings,
            ui,
            backend,
            device_lost,
            failure,
        })
    }
    /// Actual backend used by the adapter, for diagnostics and CI evidence.
    pub fn backend(&self) -> &str {
        &self.backend
    }
    /// Whether the GPU connection was lost and all its resources need recreation.
    pub fn is_device_lost(&self) -> bool {
        // Native callbacks need polling; on browser WebGPU this is a documented no-op.
        let _ = self.device.poll(wgpu::PollType::Poll);
        self.device_lost.load(Ordering::Acquire)
    }
    fn buffers(device: &wgpu::Device, assets: &PlanetAssets) -> (wgpu::Buffer, wgpu::Buffer, u32) {
        let vertex = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("planet vertices"),
            contents: bytemuck::cast_slice(&assets.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("planet indices"),
            contents: bytemuck::cast_slice(&assets.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        (vertex, index, assets.indices.len() as u32)
    }
    fn assets_bindings(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pipeline: &wgpu::RenderPipeline,
        uniform: &wgpu::Buffer,
        assets: &PlanetAssets,
    ) -> wgpu::BindGroup {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("procedural terrain"),
            size: wgpu::Extent3d {
                width: assets.texture_size,
                height: assets.texture_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            &assets.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(assets.texture_size * 4),
                rows_per_image: Some(assets.texture_size),
            },
            texture.size(),
        );
        let view = texture.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nearest terrain"),
            ..Default::default()
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("planet bindings"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        })
    }
    fn targets(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        pipeline: &wgpu::RenderPipeline,
    ) -> (wgpu::TextureView, wgpu::TextureView, wgpu::BindGroup) {
        let make = |format, usage| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("pixel target"),
                    size: wgpu::Extent3d {
                        width: (config.width / 3).max(1),
                        height: (config.height / 3).max(1),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let color = make(
            config.format,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let depth = make(
            wgpu::TextureFormat::Depth24Plus,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        let sampler = device.create_sampler(&Default::default());
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("upscale bindings"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        (color, depth, bindings)
    }
    /// Replace all world-specific assets; old resources are released by ownership.
    pub fn replace_world(&mut self, assets: &PlanetAssets) {
        if self.is_device_lost() {
            return;
        }
        (self.vertex, self.index, self.index_count) = Self::buffers(&self.device, assets);
        self.bindings = Self::assets_bindings(
            &self.device,
            &self.queue,
            &self.pipeline,
            &self.uniform,
            assets,
        );
    }
    /// Resize nonzero render targets. Minimized/suspended windows must not draw.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 || self.is_device_lost() {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        (self.low_color, self.low_depth, self.blit_bindings) =
            Self::targets(&self.device, &self.config, &self.blit_pipeline);
    }
    /// Draw the low-resolution planet, upscale it, then draw the interface.
    pub fn draw(
        &mut self,
        rotation: glam::Quat,
        distance: f32,
        selection: Option<[f32; 3]>,
        context: &egui::Context,
        output: egui::FullOutput,
    ) -> Result<bool, String> {
        if self.is_device_lost() {
            return Ok(false);
        }
        if let Some(error) = self
            .failure
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            return Err(error);
        }
        let model = glam::Mat4::from_quat(rotation);
        let view = glam::Mat4::look_at_rh(
            glam::vec3(0., 0., distance),
            glam::Vec3::ZERO,
            glam::Vec3::Y,
        );
        let projection = glam::Mat4::perspective_rh(
            38f32.to_radians(),
            self.config.width as f32 / self.config.height as f32,
            0.1,
            50.,
        );
        let uniforms = Uniforms {
            mvp: (projection * view * model).to_cols_array_2d(),
            model: model.to_cols_array_2d(),
            selection: selection.map_or([0.; 4], |p| [p[0], p[1], p[2], 1.]),
        };
        self.queue
            .write_buffer(&self.uniform, 0, bytemuck::bytes_of(&uniforms));
        // Keep atlas updates even when presentation must wait for a new surface.
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                self.ui
                    .update_texture(&self.device, &self.queue, *id, delta);
            }
        }
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.resize(self.config.width, self.config.height);
                self.free_ui_textures(&output);
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                self.free_ui_textures(&output);
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self
                    .instance
                    .create_surface(self.window.clone())
                    .map_err(|e| e.to_string())?;
                self.resize(self.config.width, self.config.height);
                self.free_ui_textures(&output);
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                self.free_ui_textures(&output);
                return Err("graphics surface validation failed".into());
            }
        };
        let target = frame.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let paint = context.tessellate(output.shapes, output.pixels_per_point);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: output.pixels_per_point,
        };
        let extra =
            self.ui
                .update_buffers(&self.device, &self.queue, &mut encoder, &paint, &screen);
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("planet pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.low_color,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.004,
                            g: 0.003,
                            b: 0.014,
                            a: 1.,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.low_depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bindings, &[]);
            pass.set_vertex_buffer(0, self.vertex.slice(..));
            pass.set_index_buffer(self.index.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.index_count, 0, 0..1);
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("presentation"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.blit_pipeline);
            pass.set_bind_group(0, &self.blit_bindings, &[]);
            pass.draw(0..3, 0..1);
            self.ui.render(&mut pass.forget_lifetime(), &paint, &screen);
        }
        self.queue
            .submit(extra.into_iter().chain([encoder.finish()]));
        self.queue.present(frame);
        for id in &output.textures_delta.free {
            self.ui.free_texture(id);
        }
        if let Some(error) = self
            .failure
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            return Err(error);
        }
        Ok(!self.is_device_lost())
    }
    fn free_ui_textures(&mut self, output: &egui::FullOutput) {
        for id in &output.textures_delta.free {
            self.ui.free_texture(id);
        }
    }
}
