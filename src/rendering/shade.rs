use wayland_client::protocol::wl_surface;
use wgpu::util::DeviceExt;

use crate::{
    runtime_data::RuntimeData,
    traits::ToLocal,
    types::{Config, Monitor, Rect, Selection},
};

const TOP_LEFT: [f32; 2] = [-1.0, 1.0];
const BOTTOM_LEFT: [f32; 2] = [-1.0, -1.0];
const TOP_RIGHT: [f32; 2] = [1.0, 1.0];
const BOTTOM_RIGHT: [f32; 2] = [1.0, -1.0];

const RECT_VERTICES: &[[f32; 2]] = &[
    TOP_RIGHT,
    TOP_LEFT,
    BOTTOM_LEFT,
    // --
    BOTTOM_LEFT,
    BOTTOM_RIGHT,
    TOP_RIGHT,
];

pub struct Overlay {
    pub render_pipeline: wgpu::RenderPipeline,
    shade_bind_group: wgpu::BindGroup,
    selection_bind_group: wgpu::BindGroup,
}

impl Overlay {
    pub fn new(device: &wgpu::Device, config: &Config) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Overlay shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../res/overlay.wgsl").into()),
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Overlay render pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Overlay render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
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

        let selection_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Selection color uniform buffer"),
            contents: bytemuck::cast_slice(&[config.selection_color]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let selection_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Selection bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: selection_buffer.as_entire_binding(),
            }],
        });

        Self {
            render_pipeline,
            shade_bind_group,
            selection_bind_group,
        }
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        texture_view: &wgpu::TextureView,
        monitor: &Monitor,
    ) {
        let color_attachment = wgpu::RenderPassColorAttachment {
            view: texture_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: true,
            },
        };
        // Draw the shade
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(color_attachment.clone())],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, monitor.shade.shade_vertex_buffer.slice(..));
            render_pass.set_bind_group(0, &self.shade_bind_group, &[]);
            render_pass.draw(0..monitor.shade.shade_vertex_count, 0..1);
        }
        // Draw the selection outline
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(color_attachment)],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, monitor.shade.selection_vertex_buffer.slice(..));
            render_pass.set_bind_group(0, &self.selection_bind_group, &[]);
            render_pass.draw(0..monitor.shade.selection_vertex_count, 0..1);
        }
    }
}

pub struct MonitorOverlay {
    shade_vertex_count: u32,
    shade_vertex_buffer: wgpu::Buffer,

    selection_vertex_count: u32,
    selection_vertex_buffer: wgpu::Buffer,
}

impl MonitorOverlay {
    pub fn new(runtime_data: &RuntimeData) -> Self {
        let shade_vertex_buffer = runtime_data.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            // 24 is the maximum amount of vertices for covering the area with triangles
            size: 24 * std::mem::size_of::<Vertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let selection_vertex_buffer = runtime_data.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            // 24 is the maximum amount of vertices for covering the area with triangles
            size: 24 * std::mem::size_of::<Vertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            shade_vertex_count: 0,
            shade_vertex_buffer,
            selection_vertex_count: 0,
            selection_vertex_buffer,
        }
    }

    pub fn update_vertices(
        &mut self,
        mon_rect: &Rect<i32>,
        wl_surface: &wl_surface::WlSurface,
        selection: &Selection,
        config: &Config,
        queue: &wgpu::Queue,
    ) {
        let (shade_vertices, selection_vertices): (Vec<[f32; 2]>, Vec<[f32; 2]>) = match selection {
            Selection::Rectangle(Some(selection)) => {
                match selection.extents.to_rect().constrain(mon_rect) {
                    None => {
                        self.shade_vertex_count = 6;
                        self.selection_vertex_count = 0;

                        (RECT_VERTICES.to_vec(), vec![])
                    }
                    Some(rect) => {
                        self.shade_vertex_count = 24;
                        self.selection_vertex_count = 24;

                        let rect = rect.to_local(mon_rect);

                        let outer = rect
                            .padded(config.line_width)
                            .to_render_space(mon_rect.width as f32, mon_rect.height as f32);

                        let rect =
                            rect.to_render_space(mon_rect.width as f32, mon_rect.height as f32);

                        (
                            Vertex::hollow_rect_vertices(&Rect::new(-1.0, 1.0, 2.0, 2.0), &rect),
                            Vertex::hollow_rect_vertices(&outer, &rect),
                        )
                    }
                }
            }
            Selection::Display(Some(selection)) => {
                if selection.wl_surface == *wl_surface {
                    self.shade_vertex_count = 0;
                    self.selection_vertex_count = 24;

                    let rect = mon_rect.to_local(mon_rect);

                    let inner = rect
                        .padded(-config.display_highlight_width)
                        .to_render_space(rect.width as f32, rect.height as f32);

                    (
                        vec![],
                        Vertex::hollow_rect_vertices(
                            &rect.to_render_space(rect.width as f32, rect.height as f32),
                            &inner,
                        ),
                    )
                } else {
                    self.shade_vertex_count = 6;
                    self.selection_vertex_count = 0;
                    (RECT_VERTICES.to_vec(), vec![])
                }
            }
            _ => {
                self.selection_vertex_count = 0;
                self.shade_vertex_count = 6;
                (RECT_VERTICES.to_vec(), vec![])
            }
        };
        queue.write_buffer(
            &self.shade_vertex_buffer,
            0,
            bytemuck::cast_slice(&shade_vertices),
        );
        queue.write_buffer(
            &self.selection_vertex_buffer,
            0,
            bytemuck::cast_slice(&selection_vertices),
        );
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 2],
}

impl Vertex {
    const ATTRS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRS,
        }
    }

    fn hollow_rect_vertices(outer: &Rect<f32>, inner: &Rect<f32>) -> Vec<[f32; 2]> {
        let top_left = [inner.x, inner.y];
        let bottom_left = [inner.x, inner.y - inner.height];
        let top_right = [inner.x + inner.width, inner.y];
        let bottom_right = [inner.x + inner.width, inner.y - inner.height];

        let outer_top_left = [outer.x, outer.y];
        let outer_bottom_left = [outer.x, outer.y - outer.height];
        let outer_top_right = [outer.x + outer.width, outer.y];
        let outer_bottom_right = [outer.x + outer.width, outer.y - outer.height];
        vec![
            outer_bottom_left,
            bottom_left,
            top_left,
            // --
            bottom_left,
            outer_bottom_left,
            outer_bottom_right,
            // --
            outer_bottom_right,
            outer_top_right,
            bottom_right,
            // --
            bottom_right,
            bottom_left,
            outer_bottom_right,
            // --
            bottom_right,
            outer_top_right,
            top_right,
            // --
            top_right,
            outer_top_right,
            outer_top_left,
            // --
            top_right,
            outer_top_left,
            top_left,
            // --
            top_left,
            outer_top_left,
            outer_bottom_left,
        ]
    }
}
