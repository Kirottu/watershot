use std::{fs, io::Cursor, process::Command};

use fontconfig::Fontconfig;
use fontdue::{Font, FontSettings};
use image::DynamicImage;

use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    reexports::client::{
        globals::GlobalList,
        protocol::{wl_keyboard, wl_pointer, wl_shm, wl_surface},
        QueueHandle,
    },
    registry::RegistryState,
    seat::{pointer::ThemedPointer, SeatState},
    shell::wlr_layer::LayerShell,
    shm::Shm,
};

use crate::{
    rendering::{background::Background, shade::Overlay},
    traits::ToLocal,
    types::{Args, ExitState, MonitorIdentification},
    Config, Monitor, Rect, Selection,
};

/// The main data worked on at runtime
pub struct RuntimeData {
    // Different wayland things
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub layer_state: LayerShell,
    pub shm_state: Shm,

    // Devices
    pub keyboard: Option<wl_keyboard::WlKeyboard>,
    pub pointer: Option<wl_pointer::WlPointer>,

    pub pointer_surface: wl_surface::WlSurface,
    pub themed_pointer: Option<ThemedPointer>,

    /// Combined area of all monitors
    pub area: Rect<i32>,
    pub selection: Selection,
    pub monitors: Vec<Monitor>,
    pub config: Config,
    // Fontdue expects a list of fonts for layouts
    pub font: Vec<Font>,
    pub image: DynamicImage,
    pub exit: ExitState,

    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub adapter: wgpu::Adapter,
    pub queue: wgpu::Queue,

    pub background: Background,
    pub overlay: Overlay,
}

impl RuntimeData {
    pub fn new(qh: &QueueHandle<Self>, globals: &GlobalList, args: &Args) -> Self {
        let output = Command::new(args.grim.as_ref().unwrap_or(&"grim".to_string()))
            .arg("-t")
            .arg("ppm")
            .arg("-")
            .output()
            .expect("Failed to run grim command!")
            .stdout;

        let image = image::io::Reader::with_format(Cursor::new(output), image::ImageFormat::Pnm)
            .decode()
            .expect("Failed to parse grim image!");

        let config = Config::load().unwrap_or_default();

        let fc = Fontconfig::new().expect("Failed to init FontConfig");

        let fc_font = fc
            .find(&config.font_family, None)
            .expect("Failed to find font");

        let font = Font::from_bytes(
            fs::read(fc_font.path).expect("Failed to load font data"),
            FontSettings::default(),
        )
        .expect("Failed to load font");

        let compositor_state =
            CompositorState::bind(globals, qh).expect("wl_compositor is not available");

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let pointer_surface = compositor_state.create_surface(qh);

        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptionsBase {
                compatible_surface: None,
                ..Default::default()
            }))
            .unwrap();

        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default(), None)).unwrap();

        // Different rendering stages
        let background = Background::new(&device);
        let overlay = Overlay::new(&device, &config);

        RuntimeData {
            registry_state: RegistryState::new(globals),
            seat_state: SeatState::new(globals, qh),
            output_state: OutputState::new(globals, qh),
            compositor_state,
            layer_state: LayerShell::bind(globals, qh).expect("layer shell is not available"),
            shm_state: Shm::bind(globals, qh).expect("wl_shm is not available"),
            selection: Selection::Rectangle(None),
            config,
            area: Rect::default(),
            monitors: Vec::new(),
            image,
            font: vec![font],
            keyboard: None,
            pointer: None,
            themed_pointer: None,
            exit: ExitState::None,
            pointer_surface,
            instance,
            adapter,
            device,
            queue,
            background,
            overlay,
        }
    }

    pub fn draw(&mut self, identification: MonitorIdentification, qh: &QueueHandle<Self>) {
        let monitor = match identification {
            MonitorIdentification::Layer(layer) => self
                .monitors
                .iter_mut()
                .find(|window| window.layer == layer)
                .unwrap(),
            MonitorIdentification::Surface(surface) => self
                .monitors
                .iter_mut()
                .find(|window| window.wl_surface == surface)
                .unwrap(),
        };

        monitor.shade.update_vertices(
            &monitor.rect,
            &monitor.wl_surface,
            &self.selection,
            &self.config,
            &self.queue,
        );

        let surface_texture = monitor.surface.get_current_texture().unwrap();
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        self.background.render(&mut encoder, &texture_view, monitor);
        self.overlay.render(&mut encoder, &texture_view, monitor);

        self.queue.submit(Some(encoder.finish()));

        monitor
            .wl_surface
            .damage(0, 0, monitor.rect.width, monitor.rect.height);
        monitor.wl_surface.frame(qh, monitor.wl_surface.clone());
        surface_texture.present();
        monitor.wl_surface.commit();

        /*let buffer = monitor.buffer.get_or_insert_with(|| {
            monitor
                .pool
                .create_buffer(
                    monitor.rect.width,
                    monitor.rect.height,
                    monitor.rect.width * 4,
                    wl_shm::Format::Argb8888,
                )
                .expect("Failed to create buffer!")
                .0
        });

        let canvas = match monitor.pool.canvas(buffer) {
            Some(canvas) => canvas,
            None => {
                let (second_buffer, canvas) = monitor
                    .pool
                    .create_buffer(
                        monitor.rect.width,
                        monitor.rect.height,
                        monitor.rect.width * 4,
                        wl_shm::Format::Argb8888,
                    )
                    .expect("Failed to create buffer!");
                *buffer = second_buffer;
                canvas
            }
        };

        // Magic to convert a &mut [u8] to &mut [u32], the length is the original length divided by the size of an u32
        let slice = unsafe {
            std::slice::from_raw_parts_mut(
                canvas.as_mut_ptr() as *mut u32,
                canvas.len() / std::mem::size_of::<u32>(),
            )
        };

        let mut target = DrawTarget::from_backing(monitor.rect.width, monitor.rect.height, slice);

        // Only draw if there is damage
        if !monitor.damage.is_empty() {
            let rect = monitor
                .damage
                .clone()
                .into_iter()
                .reduce(|mut accum, rect| {
                    accum.extend(&rect);
                    accum
                })
                .unwrap()
                .padded(self.config.handle_radius)
                .constrain(&monitor.rect)
                .unwrap()
                .to_local(&monitor.rect);

            let full_image = Image {
                width: monitor.rect.width,
                height: monitor.rect.height,
                data: &monitor.image,
            };

            let data = full_image.crop(&rect);
            let image = Image {
                width: rect.width,
                height: rect.height,
                data: &data,
            };

            target.draw_image_at(
                rect.x as f32,
                rect.y as f32,
                &image,
                &DrawOptions::default(),
            );

            match &self.selection {
                Selection::Rectangle(Some(selection)) => {
                    let mut pb = PathBuilder::new();
                    let ext = &selection.extents.to_local(&monitor.rect);
                    let selection_rect = ext.to_rect();

                    let shade_rects = rect.subtract(&selection_rect);

                    // Draw the shaded rects
                    for rect in shade_rects {
                        target.fill_rect(
                            rect.x as f32,
                            rect.y as f32,
                            rect.width as f32,
                            rect.height as f32,
                            &Source::Solid(self.config.shade_color.into()),
                            &DrawOptions::default(),
                        );
                    }

                    pb.move_to(ext.start_x as f32, ext.start_y as f32);
                    pb.line_to(ext.end_x as f32, ext.start_y as f32);
                    pb.line_to(ext.end_x as f32, ext.end_y as f32);
                    pb.line_to(ext.start_x as f32, ext.end_y as f32);
                    pb.line_to(ext.start_x as f32, ext.start_y as f32);
                    target.stroke(
                        &pb.finish(),
                        &Source::Solid(self.config.selection_color.into()),
                        &StrokeStyle {
                            width: self.config.line_width as f32,
                            ..Default::default()
                        },
                        &DrawOptions::default(),
                    );

                    // Draw the handles
                    for (x, y, _) in handles!(ext) {
                        let mut pb = PathBuilder::new();
                        pb.arc(
                            *x as f32,
                            *y as f32,
                            self.config.handle_radius as f32,
                            0.0,
                            std::f32::consts::PI * 2.0,
                        );

                        target.fill(
                            &pb.finish(),
                            &Source::Solid(self.config.selection_color.into()),
                            &DrawOptions::default(),
                        );
                    }
                }
                Selection::Display(Some(selection)) => {
                    if selection.surface == monitor.wl_surface {
                        let mut pb = PathBuilder::new();
                        pb.move_to(0.0, 0.0);
                        pb.line_to(monitor.rect.width as f32, 0.0);
                        pb.line_to(monitor.rect.width as f32, monitor.rect.height as f32);
                        pb.line_to(0.0, monitor.rect.height as f32);
                        pb.line_to(0.0, 0.0);

                        target.stroke(
                            &pb.finish(),
                            &Source::Solid(self.config.selection_color.into()),
                            &StrokeStyle {
                                width: self.config.display_highlight_width as f32 * 2.0,
                                ..Default::default()
                            },
                            &DrawOptions::default(),
                        );
                    } else {
                        target.fill_rect(
                            0.0,
                            0.0,
                            monitor.rect.width as f32,
                            monitor.rect.height as f32,
                            &Source::Solid(self.config.shade_color.into()),
                            &DrawOptions::default(),
                        );
                    }
                }
                _ => {
                    target.fill_rect(
                        0.0,
                        0.0,
                        monitor.rect.width as f32,
                        monitor.rect.height as f32,
                        &Source::Solid(self.config.shade_color.into()),
                        &DrawOptions::default(),
                    );

                    let text = match &self.selection {
                        Selection::Rectangle(_) => "RECTANGLE MODE",
                        Selection::Display(_) => "DISPLAY MODE",
                    };

                    target.draw_text(
                        monitor.rect.width as f32 / 2.0,
                        monitor.rect.height as f32 / 2.0,
                        &self.font,
                        text,
                        self.config.mode_text_size as f32,
                        self.config.text_color,
                    );
                }
            }
            monitor
                .wl_surface
                .damage_buffer(rect.x, rect.y, rect.width, rect.height);
        }

        monitor.damage.clear();

        monitor.wl_surface.frame(qh, monitor.wl_surface.clone());
        buffer
            .attach_to(&monitor.wl_surface)
            .expect("Failed to attach buffer to surface");
        monitor.wl_surface.commit();*/
    }
}
