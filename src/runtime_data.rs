use std::{fs, io::Cursor, process::Command};

use fontconfig::Fontconfig;
use image::DynamicImage;

use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    reexports::client::{
        globals::GlobalList,
        protocol::{wl_keyboard, wl_pointer, wl_surface},
        QueueHandle,
    },
    registry::RegistryState,
    seat::{pointer::ThemedPointer, SeatState},
    shell::wlr_layer::LayerShell,
    shm::Shm,
};

use crate::{
    rendering::Renderer,
    types::{Args, ExitState, MonitorIdentification, RectangleSelection},
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
    /// The scale factor of the screenshot image
    pub scale_factor: f32,
    pub selection: Selection,
    pub monitors: Vec<Monitor>,
    pub config: Config,
    pub font: wgpu_text::font::FontArc,
    pub image: DynamicImage,
    pub exit: ExitState,
    pub args: Args,

    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub adapter: wgpu::Adapter,
    pub queue: wgpu::Queue,

    pub renderer: Renderer,
}

impl RuntimeData {
    pub fn new(qh: &QueueHandle<Self>, globals: &GlobalList, args: Args) -> Self {
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

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                ..Default::default()
            },
            None,
        ))
        .unwrap();

        let renderer = Renderer::new(&device, &config);

        let selection: Selection = if let Some(rect) = args.initial_selection {
            Selection::Rectangle(Some(
                RectangleSelection {
                    extents: rect.to_extents(),
                    modifier: None,
                    active: false,
                }
            ))
        } else {
            Selection::Rectangle(None)
        };

        RuntimeData {
            registry_state: RegistryState::new(globals),
            seat_state: SeatState::new(globals, qh),
            output_state: OutputState::new(globals, qh),
            compositor_state,
            layer_state: LayerShell::bind(globals, qh).expect("layer shell is not available"),
            shm_state: Shm::bind(globals, qh).expect("wl_shm is not available"),
            selection,
            config,
            area: Rect::default(),
            monitors: Vec::new(),
            // Set later
            scale_factor: 0.0,
            image,
            keyboard: None,
            pointer: None,
            themed_pointer: None,
            exit: ExitState::None,
            args,
            pointer_surface,
            instance,
            adapter,
            device,
            queue,
            renderer,
            font: wgpu_text::font::FontArc::try_from_vec(
                fs::read(fc_font.path).expect("Failed to load font"),
            )
            .expect("Invalid font data"),
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

        monitor.rendering.update_overlay_vertices(
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

        self.renderer.render(
            &mut encoder,
            &texture_view,
            monitor,
            &self.selection,
            &self.device,
            &self.queue,
        );

        self.queue.submit(Some(encoder.finish()));

        monitor
            .wl_surface
            .damage(0, 0, monitor.rect.width, monitor.rect.height);
        monitor.wl_surface.frame(qh, monitor.wl_surface.clone());
        surface_texture.present();
        monitor.wl_surface.commit();
    }
}
