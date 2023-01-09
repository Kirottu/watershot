use std::{cmp::Ordering, env, fs};

use raqote::SolidSource;
use serde::Deserialize;
use smithay_client_toolkit::{
    reexports::client::protocol::wl_surface,
    shell::layer::LayerSurface,
    shm::slot::{Buffer, SlotPool},
};

use crate::runtime_data::RuntimeData;

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
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            shade_color: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 127,
            },
            text_color: Color {
                r: 190,
                g: 190,
                b: 190,
                a: 255,
            },
            size_text_size: 15,
            mode_text_size: 30,
            font_family: "monospace".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Copy, Clone)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl From<Color> for SolidSource {
    fn from(color: Color) -> Self {
        Self {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        }
    }
}

/// Represents the layer and the monitor it resides on
pub struct Monitor {
    pub layer: LayerSurface,
    pub image: Vec<u32>,
    pub pool: SlotPool,
    pub buffer: Option<Buffer>,
    pub rect: Rect,
    pub damage: Vec<Rect>,
}

impl Monitor {
    pub fn new(layer: LayerSurface, rect: Rect, runtime_data: &RuntimeData) -> Self {
        Self {
            layer,
            image: runtime_data
                .image
                .crop_imm(
                    (rect.x - runtime_data.area.x) as u32,
                    (rect.y - runtime_data.area.y) as u32,
                    rect.width as u32,
                    rect.height as u32,
                )
                .to_rgba8()
                .chunks_exact(4)
                .map(|chunks| {
                    SolidSource::from_unpremultiplied_argb(
                        chunks[3], chunks[0], chunks[1], chunks[2],
                    )
                    .to_u32()
                })
                .collect(),
            buffer: None,
            rect,
            pool: SlotPool::new(
                rect.width as usize * rect.height as usize * 4,
                &runtime_data.shm_state,
            )
            .expect("Failed to create pool!"),
            damage: vec![rect],
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
    pub fn to_rect(self) -> Rect {
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

    pub fn to_rect_clamped(self, area: &Rect) -> Rect {
        let mut rect = self.to_rect();

        rect.x = rect.x.clamp(area.x, area.x + area.width - rect.width);
        rect.y = rect.y.clamp(area.y, area.y + area.height - rect.height);

        rect
    }
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

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
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let width = (self.x - x + self.width).max(other.x - x + other.width);
        let height = (self.y - y + self.height).max(other.y - y + other.height);

        *self = Self::new(x, y, width, height);
    }

    pub fn padded(self, amount: i32) -> Self {
        Self {
            x: self.x - amount,
            y: self.y - amount,
            width: self.width + amount,
            height: self.height + amount,
        }
    }

    /// Return the split up rectangle, with the area provided missing
    pub fn substract(&self, subtract: &Self) -> Vec<Self> {
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
                y: self.y,
                width: subtract.x - self.x,
                height: self.height,
            });
        }

        // Add the part of the Rect to the right of the subtraction
        if self.x + self.width > subtract.x + subtract.width {
            result.push(Rect {
                x: subtract.x + subtract.width,
                y: self.y,
                width: (self.x + self.width) - (subtract.x + subtract.width),
                height: self.height,
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
        results.append(&mut self.substract(&intersection));
        results.append(&mut other.substract(&intersection));
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

            res.width = (res.width + res.x).clamp(0, area.width + area.x) - res.x;
            res.height = (res.height + res.y).clamp(0, area.height + area.y) - res.y;

            Some(res)
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
    pub surface: wl_surface::WlSurface,
}

impl DisplaySelection {
    pub fn new(surface: wl_surface::WlSurface) -> Self {
        Self { surface }
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
    ExitWithSelection(Rect),
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
