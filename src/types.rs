use std::{env, fs, io::Cursor, process::Command};

use clap::{Parser, Subcommand};
use image::DynamicImage;
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
    WaylandDisplayHandle, WaylandWindowHandle,
};
use serde::Deserialize;
use smithay_client_toolkit::{
    output::OutputInfo,
    shell::{
        wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerSurface},
        WaylandSurface,
    },
};
use wayland_client::{
    protocol::{wl_output, wl_surface},
    Connection, Proxy, QueueHandle,
};

use crate::{rendering::MonSpecificRendering, runtime_data::RuntimeData};

#[cfg(feature = "window-selection")]
use crate::window::{search::WindowSearchParam, DescribesWindow, WindowDescriptor};

#[derive(Parser, Clone, Debug)]
#[command(author, version, about)]
pub struct Args {
    /// Copy the screenshot after exit
    #[arg(short, long)]
    pub copy: bool,

    /// Output the screenshot into stdout in PNG format
    #[arg(short, long)]
    pub stdout: bool,

    /// Path to the `grim` executable
    #[arg(short, long)]
    pub grim: Option<String>,

    /// Save the image into a file
    #[command(subcommand)]
    pub save: Option<SaveLocation>,

    /// Pre-selects a window by its class, title or initial versions of the two.
    /// The value passed can be a regex.
    /// Examples: "class=Alacritty" , "title=.*Visual Studio Code.*"
    #[cfg(feature = "window-selection")]
    #[arg(long, group = "capture-window")]
    pub window_search: Option<WindowSearchParam>,

    /// Pre-selects the window under the mouse cursor.
    #[cfg(feature = "window-selection")]
    #[arg(long, group = "capture-window")]
    pub window_under_cursor: bool,

    /// Pre-selects the currently-focused window.
    #[cfg(feature = "window-selection")]
    #[arg(long, group = "capture-window")]
    pub active_window: bool,

    /// Automatically captures the pre-selected window, skipping interactive mode.
    #[cfg(feature = "window-selection")]
    #[arg(long)]
    pub auto_capture: bool,
}

#[derive(Subcommand, Clone, Debug)]
pub enum SaveLocation {
    /// The path to save the image to
    Path { path: String },
    /// The directory to save the image to with a generated name
    Directory { path: String },
}

/// The configuration for colors and other things like that
#[derive(Debug, Deserialize)]
pub struct Config {
    pub handle_radius: i32,
    pub line_width: i32,
    pub display_highlight_width: i32,
    pub selection_color: Color,
    pub shade_color: Color,
    pub text_color: Color,
    pub mode_text_size: i32,
    pub font_family: String,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let string = fs::read_to_string(format!("{}/.config/watershot.ron", env::var("HOME")?))?;
        Ok(ron::from_str(&string)?)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            handle_radius: 10,
            line_width: 1,
            display_highlight_width: 5,
            selection_color: Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            },
            shade_color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.5,
            },
            text_color: Color {
                r: 0.8,
                g: 0.8,
                b: 0.8,
                a: 1.0,
            },
            mode_text_size: 30,
            font_family: "monospace".to_string(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Deserialize, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl From<Color> for wgpu_text::glyph_brush::Color {
    fn from(val: Color) -> Self {
        [val.r, val.g, val.b, val.a]
    }
}

/// Represents the layer and the monitor it resides on
pub struct Monitor {
    pub layer: LayerSurface,
    pub wl_surface: wl_surface::WlSurface,
    pub surface: wgpu::Surface,
    pub rect: Rect<i32>,
    pub image: DynamicImage,
    /// The wayland scale factor for this monitor
    pub scale_factor: i32,
    pub rendering: MonSpecificRendering,
}

impl Monitor {
    pub fn new(
        rect: Rect<i32>,
        qh: &QueueHandle<RuntimeData>,
        conn: &Connection,
        output: wl_output::WlOutput,
        info: OutputInfo,
        runtime_data: &RuntimeData,
    ) -> Self {
        let wl_surface = runtime_data.compositor_state.create_surface(qh);

        let layer = runtime_data.layer_state.create_layer_surface(
            qh,
            wl_surface.clone(),
            Layer::Overlay,
            Some("watershot"),
            Some(&output),
        );

        // Set the right scale for the buffer
        wl_surface.set_buffer_scale(info.scale_factor);

        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);

        layer.commit();

        let handle = RawWgpuHandles::new(conn, &wl_surface);

        // Each monitor also gets their own screenshot to preserve clarity as much as possible
        let grim_output = Command::new(
            runtime_data
                .args
                .grim
                .as_ref()
                .unwrap_or(&"grim".to_string()),
        )
        .arg("-t")
        .arg("ppm")
        .arg("-o")
        .arg(info.name.as_ref().unwrap())
        .arg("-")
        .output()
        .expect("Failed to run grim command!")
        .stdout;

        let image =
            image::io::Reader::with_format(Cursor::new(grim_output), image::ImageFormat::Pnm)
                .decode()
                .expect("Failed to parse grim image!");

        let surface = unsafe { runtime_data.instance.create_surface(&handle).unwrap() };
        let rendering = MonSpecificRendering::new(&rect, &info, image.to_rgba8(), runtime_data);

        Self {
            layer,
            wl_surface,
            rect,
            image,
            scale_factor: info.scale_factor,
            surface,
            rendering,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Extents {
    pub start_x: i32,
    pub start_y: i32,
    pub end_x: i32,
    pub end_y: i32,
}

impl Extents {
    pub fn to_rect(self) -> Rect<i32> {
        let (x, width) = if self.start_x < self.end_x {
            (self.start_x, self.end_x - self.start_x)
        } else {
            (self.end_x, self.start_x - self.end_x)
        };

        let (y, height) = if self.start_y < self.end_y {
            (self.start_y, self.end_y - self.start_y)
        } else {
            (self.end_y, self.start_y - self.end_y)
        };
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    pub fn to_rect_clamped(self, area: &Rect<i32>) -> Rect<i32> {
        let mut rect = self.to_rect();

        rect.x = rect.x.clamp(area.x, area.x + area.width - rect.width);
        rect.y = rect.y.clamp(area.y, area.y + area.height - rect.height);

        rect
    }
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct Rect<T> {
    pub x: T,
    pub y: T,
    pub width: T,
    pub height: T,
}

impl<T> Rect<T> {
    pub fn new(x: T, y: T, width: T, height: T) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

impl Rect<i32> {
    pub fn intersects(&self, other: &Self) -> bool {
        ((self.x + self.width).min(other.x + other.width) - self.x.max(other.x)) > 0
            && ((self.y + self.height).min(other.y + other.height) - self.y.max(other.y)) > 0
    }

    pub fn to_extents(self) -> Extents {
        Extents {
            start_x: self.x,
            start_y: self.y,
            end_x: self.x + self.width,
            end_y: self.y + self.height,
        }
    }

    pub fn extend(&mut self, other: &Self) {
        if *self == Self::default() {
            *self = *other;
            return;
        }

        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let width = (self.x - x + self.width).max(other.x - x + other.width);
        let height = (self.y - y + self.height).max(other.y - y + other.height);

        *self = Self::new(x, y, width, height);
    }

    /// Constrain the rectangle to fit inside the provided rectangle
    pub fn constrain(&self, area: &Self) -> Option<Self> {
        if !self.intersects(area) {
            None
        } else {
            let mut res = *self;

            res.x = res.x.max(area.x);
            res.y = res.y.max(area.y);

            res.width = (self.x + self.width - res.x).clamp(0, area.width);
            res.height = (self.y + self.height - res.y).clamp(0, area.height);

            Some(res)
        }
    }

    pub fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
}

#[derive(Debug, Copy, Clone)]
pub enum SelectionModifier {
    Left,
    Right,
    Top,
    Bottom,
    TopRight,
    BottomRight,
    BottomLeft,
    TopLeft,
    // Offset from top left corner and original extents
    Center(i32, i32, Extents),
}

#[derive(Debug, Clone)]
pub enum Selection {
    Rectangle(Option<RectangleSelection>),
    Display(Option<DisplaySelection>),
    #[cfg(feature = "window-selection")]
    Window(Option<WindowDescriptor>),
}

impl Selection {
    pub fn flattened(&self) -> Selection {
        match self {
            #[cfg(feature = "window-selection")]
            Self::Window(Some(window)) => Self::Rectangle(Some(RectangleSelection {
                extents: window.get_window_rect().to_extents(),
                modifier: None,
                active: false,
            })),
            #[cfg(feature = "window-selection")]
            Self::Window(None) => Self::Rectangle(None),
            _ => self.clone(),
        }
    }

    #[cfg(feature = "window-selection")]
    pub fn from_window(window: Option<WindowDescriptor>) -> Self {
        match window {
            Some(window) => Self::Window(Some(window)),
            None => Self::Rectangle(None),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RectangleSelection {
    pub extents: Extents,
    pub modifier: Option<SelectionModifier>,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct DisplaySelection {
    pub wl_surface: wl_surface::WlSurface,
}

impl DisplaySelection {
    pub fn new(surface: wl_surface::WlSurface) -> Self {
        Self {
            wl_surface: surface,
        }
    }
}

impl RectangleSelection {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            extents: Extents {
                start_x: x,
                start_y: y,
                end_x: x,
                end_y: y,
            },
            modifier: None,
            active: true,
        }
    }
}

pub enum MonitorIdentification {
    Layer(LayerSurface),
    Surface(wl_surface::WlSurface),
}

pub enum ExitState {
    /// Not going to exit
    None,
    /// Only exit
    ExitOnly,
    /// Exit and perform actions on the selection
    ExitWithSelection(Rect<i32>),
}

pub struct RawWgpuHandles {
    window: RawWindowHandle,
    display: RawDisplayHandle,
}

impl RawWgpuHandles {
    pub fn new(conn: &Connection, surface: &wl_surface::WlSurface) -> Self {
        let mut display_handle = WaylandDisplayHandle::empty();
        display_handle.display = conn.backend().display_ptr() as *mut _;

        let mut window_handle = WaylandWindowHandle::empty();
        window_handle.surface = surface.id().as_ptr() as *mut _;

        Self {
            window: RawWindowHandle::Wayland(window_handle),
            display: RawDisplayHandle::Wayland(display_handle),
        }
    }
}

unsafe impl HasRawWindowHandle for RawWgpuHandles {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.window
    }
}

unsafe impl HasRawDisplayHandle for RawWgpuHandles {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        self.display
    }
}
