use std::{env, fs};

use clap::{Parser, Subcommand};
use image::GenericImageView;
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
    WaylandDisplayHandle, WaylandWindowHandle,
};
use serde::Deserialize;
use smithay_client_toolkit::{
    reexports::client::{protocol::wl_surface, Connection},
    shell::wlr_layer::LayerSurface,
    shm::slot::{Buffer, SlotPool},
};
use wayland_client::Proxy;

use crate::{
    rendering::{
        background::{Background, MonitorBackground},
        shade::MonitorOverlay,
    },
    runtime_data::RuntimeData,
};

#[derive(Parser)]
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
}

#[derive(Subcommand)]
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
    pub size_text_size: i32,
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
            size_text_size: 15,
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

/// Represents the layer and the monitor it resides on
pub struct Monitor {
    pub layer: LayerSurface,
    pub wl_surface: wl_surface::WlSurface,
    pub rect: Rect<i32>,
    pub damage: Vec<Rect<i32>>,

    // Wgpu stuff
    pub surface: wgpu::Surface,

    pub shade: MonitorOverlay,
    pub background: MonitorBackground,
}

impl Monitor {
    pub fn new(
        layer: LayerSurface,
        wl_surface: wl_surface::WlSurface,
        surface: wgpu::Surface,
        rect: Rect<i32>,
        runtime_data: &RuntimeData,
    ) -> Self {
        let background = MonitorBackground::new(&rect, runtime_data);
        let shade = MonitorOverlay::new(runtime_data);

        Self {
            layer,
            wl_surface,
            rect,
            damage: vec![rect],
            surface,
            background,
            shade,
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

impl Rect<f32> {
    pub fn padded(self, amount: f32) -> Self {
        let mut width = self.width + 2.0 * amount;
        let mut height = self.height + 2.0 * amount;

        // Make sure we have no negative size
        if width < 0.0 {
            width = 0.0;
        }

        if height < 0.0 {
            height = 0.0;
        }

        Self::new(self.x - amount, self.y - amount, width, height)
    }
}

impl Rect<i32> {
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }

    pub fn intersects(&self, other: &Self) -> bool {
        ((self.x + self.width).min(other.x + other.width) - self.x.max(other.x)) > 0
            && ((self.y + self.height).min(other.y + other.height) - self.y.max(other.y)) > 0
    }

    pub fn intersecting_rect(&self, other: &Self) -> Self {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let width = (self.x + self.width).min(other.x + other.width) - x;
        let height = (self.y + self.height).min(other.y + other.height) - y;

        Self::new(x, y, width, height)
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

    pub fn padded(self, amount: i32) -> Self {
        let mut width = self.width + 2 * amount;
        let mut height = self.height + 2 * amount;

        // Make sure we have no negative size
        if width < 0 {
            width = 0;
        }

        if height < 0 {
            height = 0;
        }

        Self::new(self.x - amount, self.y - amount, width, height)
    }

    /// Return the split up rectangle, with the area provided missing
    pub fn subtract(&self, subtract: &Self) -> Vec<Self> {
        let mut result = Vec::new();

        // Add the part of the Rect above the intersection
        if self.y < subtract.y {
            result.push(Rect {
                x: self.x,
                y: self.y,
                width: self.width,
                height: subtract.y - self.y,
            });
        }

        // Add the part of the Rect below the subtraction
        if self.y + self.height > subtract.y + subtract.height {
            result.push(Rect {
                x: self.x,
                y: subtract.y + subtract.height,
                width: self.width,
                height: (self.y + self.height) - (subtract.y + subtract.height),
            });
        }

        // Add the part of the Rect to the left of the subtraction
        if self.x < subtract.x {
            result.push(Rect {
                x: self.x,
                y: subtract.y,
                width: subtract.x - self.x,
                height: subtract.height,
            });
        }

        // Add the part of the Rect to the right of the subtraction
        if self.x + self.width > subtract.x + subtract.width {
            result.push(Rect {
                x: subtract.x + subtract.width,
                y: subtract.y,
                width: (self.x + self.width) - (subtract.x + subtract.width),
                height: subtract.height,
            });
        }

        result
    }

    pub fn xor(&self, other: &Self) -> Vec<Self> {
        // If there is no intersection, the damaged area is both rects
        if !self.intersects(other) {
            return vec![*self, *other];
        }

        let intersection = self.intersecting_rect(other);

        let mut results = Vec::new();
        results.append(&mut self.subtract(&intersection));
        results.append(&mut other.subtract(&intersection));
        results
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

    pub fn to_render_space(self, width: f32, height: f32) -> Rect<f32> {
        Rect {
            x: (self.x as f32 / width - 0.5) * 2.0,
            y: -(self.y as f32 / height - 0.5) * 2.0,
            width: (self.width as f32 / width) * 2.0,
            height: (self.height as f32 / height) * 2.0,
        }
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

pub enum Selection {
    Rectangle(Option<RectangleSelection>),
    Display(Option<DisplaySelection>),
}

pub struct RectangleSelection {
    pub extents: Extents,
    pub modifier: Option<SelectionModifier>,
    pub active: bool,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intersection_rect() {
        assert_eq!(
            Rect::intersecting_rect(&Rect::new(0, 0, 10, 10), &Rect::new(5, 5, 10, 10),),
            Rect::new(5, 5, 5, 5),
        );
        assert_eq!(
            Rect::intersecting_rect(&Rect::new(0, 0, 10, 10), &Rect::new(-5, -5, 10, 10),),
            Rect::new(0, 0, 5, 5),
        );
    }

    #[test]
    fn test_xor() {
        let damage = Rect::new(0, 0, 10, 10).xor(&Rect::new(0, 0, 8, 8));

        for x in 0..15 {
            for y in 0..15 {
                if damage.iter().any(|rect| rect.contains(x, y)) {
                    print!("#");
                } else {
                    print!(".");
                }
            }
            println!();
        }
    }
}
