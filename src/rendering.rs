use image::RgbaImage;
use smithay_client_toolkit::output::OutputInfo;
use wgpu::util::DeviceExt;
use wgpu_text::glyph_brush::{HorizontalAlign, Layout, OwnedSection, OwnedText, VerticalAlign};

use crate::{
    handles,
    runtime_data::RuntimeData,
    traits::{Padded, ToLocal, ToRender},
    types::{Config, Monitor, Rect, Selection},
};

use wayland_client::protocol::wl_surface;

const TOP_LEFT: [f32; 2] = [-1.0, 1.0];
const BOTTOM_LEFT: [f32; 2] = [-1.0, -1.0];
const TOP_RIGHT: [f32; 2] = [1.0, 1.0];
const BOTTOM_RIGHT: [f32; 2] = [1.0, -1.0];

const RECT_VERTICES: &[[f32; 2]] = &[TOP_RIGHT, TOP_LEFT, BOTTOM_LEFT, BOTTOM_RIGHT];

const RECT_INDICES: &[u32] = &[0, 1, 2, 0, 2, 3];

pub const CIRCLE_EDGES: u32 = 64;
// 3 indices per edge/triangle
// 8 circles per selection highlight
// 24 indices from the selection highlight rectangle
const MAX_SEL_INDICES: u64 = CIRCLE_EDGES as u64 * 3 * 8 + 24;

const OVERLAY_MSAA: u32 = 4;

pub struct Renderer {
    // Pipelines
    tex_pipeline: wgpu::RenderPipeline,
    tex_layout: wgpu::BindGroupLayout,
    tex_sampler: wgpu::Sampler,
    tex_vertex_buffer: wgpu::Buffer,

    overlay_pipeline: wgpu::RenderPipeline,
    shade_bind_group: wgpu::BindGroup,
    sel_bind_group: wgpu::BindGroup,
}

/// Monitor specific rendering related items
pub struct MonSpecificRendering {
    /// Bind group for the background texture
    bg_bind_group: wgpu::BindGroup,

    shade_index_count: u32,
    shade_vertex_buffer: wgpu::Buffer,
    shade_index_buffer: wgpu::Buffer,

    sel_index_count: u32,
    sel_vertex_buffer: wgpu::Buffer,
    sel_index_buffer: wgpu::Buffer,

    /// Texture to render the overlay with anti-aliasing
    ms_tex: wgpu::TextureView,
    /// The target to resolve to when rendering the multisampled overlay
    ms_resolve_target_tex: wgpu::TextureView,
    /// Bind group for the resolve target texture
    ms_bind_group: wgpu::BindGroup,

    pub brush: wgpu_text::TextBrush<wgpu_text::glyph_brush::ab_glyph::FontArc>,
    rect_mode_section: OwnedSection,
    display_mode_section: OwnedSection,
    window_mode_section: OwnedSection,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, config: &Config, format: wgpu::TextureFormat) -> Self {
        let tex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Background shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../res/texture.wgsl").into()),
        });

        let tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
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
            label: Some("Background bind group layout"),
        });

        let tex_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Background pipeline layout"),
            bind_group_layouts: &[&tex_layout],
            push_constant_ranges: &[],
        });

        let tex_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Background pipeline"),
            layout: Some(&tex_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &tex_shader,
                entry_point: "vs_main",
                buffers: &[TexVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &tex_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let tex_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let tex_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Background vertex buffer"),
            contents: bytemuck::cast_slice(TexVertex::RECT_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let color_shapes_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Color shape shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../res/color_shapes.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Overlay bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let overlay_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Overlay render pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let overlay_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Overlay render pipeline"),
            layout: Some(&overlay_layout),
            vertex: wgpu::VertexState {
                module: &color_shapes_shader,
                entry_point: "vs_main",
                buffers: &[OverlayVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &color_shapes_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: OVERLAY_MSAA,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let shade_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Shade color uniform buffer"),
            contents: bytemuck::cast_slice(&[config.shade_color]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let shade_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shade bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: shade_buffer.as_entire_binding(),
            }],
        });

        let sel_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Selection color uniform buffer"),
            contents: bytemuck::cast_slice(&[config.selection_color]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let sel_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Selection bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: sel_buffer.as_entire_binding(),
            }],
        });

        Self {
            tex_pipeline,
            tex_layout,
            tex_sampler,
            tex_vertex_buffer,
            overlay_pipeline,
            shade_bind_group,
            sel_bind_group,
        }
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        monitor: &mut Monitor,
        selection: &Selection,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let Some(rendering) = &mut monitor.rendering else {
            return
        };
        // Render the screenshot as the background
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(&self.tex_pipeline);
            render_pass.set_vertex_buffer(0, self.tex_vertex_buffer.slice(..));
            render_pass.set_bind_group(0, &rendering.bg_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }
        // Draw the shade to the multisampling texture
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &rendering.ms_tex,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(&self.overlay_pipeline);
            render_pass.set_vertex_buffer(0, rendering.shade_vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                rendering.shade_index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.set_bind_group(0, &self.shade_bind_group, &[]);
            render_pass.draw_indexed(0..rendering.shade_index_count, 0, 0..1);
        }
        // Draw the selection outline to the multisampling texture, and resolve it to the resolve texture
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &rendering.ms_tex,
                    resolve_target: Some(&rendering.ms_resolve_target_tex),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: false,
                    },
                })],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(&self.overlay_pipeline);
            render_pass.set_vertex_buffer(0, rendering.sel_vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                rendering.sel_index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.set_bind_group(0, &self.sel_bind_group, &[]);
            render_pass.draw_indexed(0..rendering.sel_index_count, 0, 0..1);
        }
        // Draw the resolve target texture on top
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(&self.tex_pipeline);
            render_pass.set_vertex_buffer(0, self.tex_vertex_buffer.slice(..));
            render_pass.set_bind_group(0, &rendering.ms_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        if let Some(section) = match selection {
            Selection::Rectangle(None) => Some(&rendering.rect_mode_section),
            Selection::Display(None) => Some(&rendering.display_mode_section),
            Selection::Window(None) => Some(&rendering.window_mode_section),
            _ => None,
        } {
            rendering.brush.queue(device, queue, vec![section]).unwrap();

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            rendering.brush.draw(&mut render_pass);
        }
    }
}

impl MonSpecificRendering {
    pub fn new(
        rect: &Rect<i32>,
        info: &OutputInfo,
        format: wgpu::TextureFormat,
        background: RgbaImage,
        runtime_data: &RuntimeData,
    ) -> Self {
        let bg_tex_size = wgpu::Extent3d {
            width: background.width(),
            height: background.height(),
            depth_or_array_layers: 1,
        };

        let bg_tex = runtime_data
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: bg_tex_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

        runtime_data.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &bg_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &background,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * background.width()),
                rows_per_image: Some(background.height()),
            },
            bg_tex_size,
        );

        let bg_tex_view = bg_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let bg_bind_group = runtime_data
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &runtime_data.renderer.as_ref().unwrap().tex_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&bg_tex_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(
                            &runtime_data.renderer.as_ref().unwrap().tex_sampler,
                        ),
                    },
                ],
            });

        let shade_vertex_buffer = runtime_data.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 8 * std::mem::size_of::<OverlayVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sel_vertex_buffer = runtime_data.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: MAX_SEL_INDICES * std::mem::size_of::<OverlayVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sel_index_buffer = runtime_data.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: MAX_SEL_INDICES * std::mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shade_index_buffer = runtime_data.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 24 * std::mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let ms_size = wgpu::Extent3d {
            width: (rect.width * info.scale_factor) as u32,
            height: (rect.height * info.scale_factor) as u32,
            depth_or_array_layers: 1,
        };

        let ms_tex = runtime_data
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: ms_size,
                mip_level_count: 1,
                sample_count: OVERLAY_MSAA,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());

        let ms_resolve_target_tex = runtime_data
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: ms_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());

        let ms_bind_group = runtime_data
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &runtime_data.renderer.as_ref().unwrap().tex_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&ms_resolve_target_tex),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(
                            &runtime_data.renderer.as_ref().unwrap().tex_sampler,
                        ),
                    },
                ],
            });

        let brush = wgpu_text::BrushBuilder::using_font(runtime_data.font.clone()).build(
            &runtime_data.device,
            (rect.width * info.scale_factor) as u32,
            (rect.height * info.scale_factor) as u32,
            format,
        );
        let pos = (
            (rect.width * info.scale_factor) as f32 / 2.0,
            (rect.height * info.scale_factor) as f32 / 2.0,
        );
        let layout = Layout::default()
            .h_align(HorizontalAlign::Center)
            .v_align(VerticalAlign::Center);

        let rect_mode_section = OwnedSection::default()
            .add_text(
                OwnedText::new("RECTANGLE MODE")
                    .with_scale((runtime_data.config.mode_text_size * info.scale_factor) as f32)
                    .with_color(runtime_data.config.text_color),
            )
            .with_layout(layout)
            .with_screen_position(pos);
        let display_mode_section = OwnedSection::default()
            .add_text(
                OwnedText::new("DISPLAY MODE")
                    .with_scale((runtime_data.config.mode_text_size * info.scale_factor) as f32)
                    .with_color(runtime_data.config.text_color),
            )
            .with_layout(layout)
            .with_screen_position(pos);

        let window_mode_section = OwnedSection::default()
            .add_text(
                OwnedText::new("WINDOW MODE")
                    .with_scale((runtime_data.config.mode_text_size * info.scale_factor) as f32)
                    .with_color(runtime_data.config.text_color),
            )
            .with_layout(layout)
            .with_screen_position(pos);

        Self {
            bg_bind_group,
            shade_vertex_buffer,
            shade_index_buffer,
            sel_vertex_buffer,
            sel_index_buffer,
            ms_tex,
            ms_resolve_target_tex,
            ms_bind_group,
            brush,
            rect_mode_section,
            display_mode_section,
            window_mode_section,
            shade_index_count: 0,
            sel_index_count: 0,
        }
    }

    pub fn update_overlay_vertices(
        &mut self,
        mon_rect: &Rect<i32>,
        wl_surface: &wl_surface::WlSurface,
        selection: &Selection,
        config: &Config,
        queue: &wgpu::Queue,
    ) {
        let flatten_selection = selection.flattened();

        let (shade_vertices, shade_indices, sel_vertices, sel_indices): (
            Vec<[f32; 2]>,
            Vec<u32>,
            Vec<[f32; 2]>,
            Vec<u32>,
        ) = match flatten_selection {
            Selection::Rectangle(Some(selection)) => {
                match selection.extents.to_rect().constrain(mon_rect) {
                    None => {
                        self.shade_index_count = 6;
                        self.sel_index_count = 0;

                        (
                            RECT_VERTICES.to_vec(),
                            RECT_INDICES.to_vec(),
                            vec![],
                            vec![],
                        )
                    }
                    Some(rect) => {
                        let rect = rect.to_local(mon_rect);

                        let outer = rect
                            .padded(config.line_width as f32 / 2.0)
                            .to_render(mon_rect.width, mon_rect.height);
                        let inner = rect
                            .padded(-config.line_width as f32 / 2.0)
                            .to_render(mon_rect.width, mon_rect.height);

                        let rect = rect.to_render(mon_rect.width, mon_rect.height);

                        let (mut sel_vertices, mut sel_indices) =
                            OverlayVertex::hollow_rect_vertices(&outer, &inner);
                        let (shade_vertices, shade_indices) = OverlayVertex::hollow_rect_vertices(
                            &Rect::new(-1.0, 1.0, 2.0, 2.0),
                            &rect,
                        );

                        let handles = handles!(selection.extents.to_local(mon_rect));

                        for (x, y, _) in handles {
                            let (mut vertices, mut indices) =
                                Circle::new(*x, *y, config.handle_radius)
                                    .to_vertices(mon_rect.width, mon_rect.height);

                            for index in &mut indices {
                                *index += sel_vertices.len() as u32;
                            }

                            sel_vertices.append(&mut vertices);
                            sel_indices.append(&mut indices);
                        }

                        self.shade_index_count = shade_indices.len() as u32;
                        self.sel_index_count = sel_indices.len() as u32;

                        (shade_vertices, shade_indices, sel_vertices, sel_indices)
                    }
                }
            }
            Selection::Display(Some(selection)) => {
                if selection.wl_surface == *wl_surface {
                    self.shade_index_count = 0;
                    self.sel_index_count = 24;

                    let rect = mon_rect.to_local(mon_rect);

                    let inner = rect
                        .padded(-config.display_highlight_width)
                        .to_render(rect.width, rect.height);

                    let (vertices, indices) = OverlayVertex::hollow_rect_vertices(
                        &rect.to_render(rect.width, rect.height),
                        &inner,
                    );

                    (vec![], vec![], vertices, indices)
                } else {
                    self.shade_index_count = 6;
                    self.sel_index_count = 0;
                    (
                        RECT_VERTICES.to_vec(),
                        RECT_INDICES.to_vec(),
                        vec![],
                        vec![],
                    )
                }
            }
            _ => {
                self.sel_index_count = 0;
                self.shade_index_count = 6;
                (
                    RECT_VERTICES.to_vec(),
                    RECT_INDICES.to_vec(),
                    vec![],
                    vec![],
                )
            }
        };
        queue.write_buffer(
            &self.shade_vertex_buffer,
            0,
            bytemuck::cast_slice(&shade_vertices),
        );
        queue.write_buffer(
            &self.sel_vertex_buffer,
            0,
            bytemuck::cast_slice(&sel_vertices),
        );
        queue.write_buffer(
            &self.sel_index_buffer,
            0,
            bytemuck::cast_slice(&sel_indices),
        );
        queue.write_buffer(
            &self.shade_index_buffer,
            0,
            bytemuck::cast_slice(&shade_indices),
        );
    }
}

#[derive(Clone, Copy)]
pub struct Circle {
    pub x: i32,
    pub y: i32,
    pub radius: i32,
}

impl Circle {
    fn new(x: i32, y: i32, radius: i32) -> Self {
        Self { x, y, radius }
    }

    fn to_vertices(self, width: i32, height: i32) -> (Vec<[f32; 2]>, Vec<u32>) {
        let mut vertices = vec![
            [self.x as f32, self.y as f32],
            [(self.x - self.radius) as f32, self.y as f32],
        ];

        let step = self.radius as f32 * 4.0 / CIRCLE_EDGES as f32;
        for i in 1..CIRCLE_EDGES / 2 {
            let offset = i as f32 * step;

            let mut x = (self.x - self.radius) as f32 + offset;
            let distance_to_center = x - self.x as f32;
            let fract = distance_to_center.abs() / self.radius as f32;

            let adjustment = (self.radius as f32 - distance_to_center.abs()) * fract;

            if distance_to_center < 0.0 {
                x -= adjustment;
            } else {
                x += adjustment;
            }

            let y = (self.radius.pow(2) as f32 - (x - self.x as f32).abs().powi(2)).sqrt();
            vertices.push([x, self.y as f32 + y]);
            vertices.push([x, self.y as f32 - y]);
        }

        vertices.push([(self.x + self.radius) as f32, self.y as f32]);

        #[rustfmt::skip]
        let mut indices = vec![
            // Leftmost triangles
            0, 1, 2,
            0, 1, 3,

            // Rightmost triangles
            0, CIRCLE_EDGES, CIRCLE_EDGES - 1,
            0, CIRCLE_EDGES, CIRCLE_EDGES - 2,
        ];

        for i in 0..CIRCLE_EDGES - 4 {
            indices.extend(&[0, i + 2, i + 4]);
        }

        (
            vertices
                .into_iter()
                .map(|vertex| vertex.to_render(width, height))
                .collect(),
            indices,
        )
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TexVertex {
    position: [f32; 2],
    tex_pos: [f32; 2],
}

impl TexVertex {
    const RECT_VERTICES: &'static [Self] = &[
        // Upper left triangle
        Self {
            position: TOP_LEFT,
            tex_pos: [0.0, 0.0],
        },
        Self {
            position: BOTTOM_LEFT,
            tex_pos: [0.0, 1.0],
        },
        Self {
            position: TOP_RIGHT,
            tex_pos: [1.0, 0.0],
        },
        // Lower right triangle
        Self {
            position: BOTTOM_RIGHT,
            tex_pos: [1.0, 1.0],
        },
        Self {
            position: TOP_RIGHT,
            tex_pos: [1.0, 0.0],
        },
        Self {
            position: BOTTOM_LEFT,
            tex_pos: [0.0, 1.0],
        },
    ];

    const ATTRS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRS,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct OverlayVertex {
    pos: [f32; 2],
}

impl OverlayVertex {
    const ATTRS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRS,
        }
    }

    fn hollow_rect_vertices(outer: &Rect<f32>, inner: &Rect<f32>) -> (Vec<[f32; 2]>, Vec<u32>) {
        let top_left = [inner.x, inner.y];
        let bottom_left = [inner.x, inner.y - inner.height];
        let top_right = [inner.x + inner.width, inner.y];
        let bottom_right = [inner.x + inner.width, inner.y - inner.height];

        let outer_top_left = [outer.x, outer.y];
        let outer_bottom_left = [outer.x, outer.y - outer.height];
        let outer_top_right = [outer.x + outer.width, outer.y];
        let outer_bottom_right = [outer.x + outer.width, outer.y - outer.height];
        (
            vec![
                top_left,
                bottom_left,
                top_right,
                bottom_right,
                outer_top_left,
                outer_bottom_left,
                outer_top_right,
                outer_bottom_right,
            ],
            vec![
                5, 1, 0, 1, 5, 7, 7, 6, 3, 3, 1, 7, 3, 6, 2, 2, 6, 4, 2, 4, 0, 0, 4, 5,
            ],
        )
    }
}
