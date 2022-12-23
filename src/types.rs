use std::{collections::HashMap, env, fs};

use serde::Deserialize;

/// Main struct for the data
#[derive(Debug)]
pub struct RuntimeData {
    pub selection: Selection,
    pub args: Args,
    pub config: Config,
    pub area_rect: Option<Rect>,
    pub windows: HashMap<gtk::ApplicationWindow, WindowInfo>,
}

/// The different selection types
#[derive(Debug)]
pub enum Selection {
    Rectangle(Option<RectangleSelection>),
    Display(Option<DisplaySelection>),
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
                r: 0.75,
                g: 0.75,
                b: 0.75,
                a: 1.0,
            },
            size_text_size: 15,
            mode_text_size: 30,
            font_family: "monospace".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

#[derive(Debug, Default)]
pub struct Args {
    pub stdout: bool,
    pub path: Option<String>,
    pub grim: Option<String>,
}

#[derive(Debug)]
pub struct DisplaySelection {
    pub window: gtk::ApplicationWindow,
}

impl DisplaySelection {
    pub fn new(window: gtk::ApplicationWindow) -> Self {
        Self { window }
    }
}

#[derive(Debug)]
pub struct RectangleSelection {
    pub extents: Extents,
    pub modifier: Option<SelectionModifier>,
    pub active: bool,
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

#[derive(Debug)]
pub struct WindowInfo {
    pub selection_overlay: gtk::DrawingArea,
    pub monitor: gdk::Monitor,
}

#[derive(Debug, Copy, Clone)]
pub struct Extents {
    pub start_x: i32,
    pub start_y: i32,
    pub end_x: i32,
    pub end_y: i32,
}

impl Extents {
    pub fn to_rect(&self) -> Rect {
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

    pub fn to_rect_clamped(&self, area: &Rect) -> Rect {
        let mut rect = self.to_rect();

        rect.x = rect.x.clamp(area.x, area.x + area.width - rect.width);
        rect.y = rect.y.clamp(area.y, area.y + area.height - rect.height);

        rect
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x > self.x && x < self.x + self.width && y > self.y && y < self.y + self.height
    }

    pub fn intersects(&self, other: &Self) -> bool {
        ((self.x + self.width).min(other.x + other.width) - self.x.max(other.x)) > 0
            && ((self.y + self.height).min(other.y + other.height) - self.y.max(other.y)) > 0
    }

    pub fn to_extents(&self) -> Extents {
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

        *self = Rect {
            x,
            y,
            width,
            height,
        };
    }
}

impl From<gtk::Rectangle> for Rect {
    fn from(rect: gtk::Rectangle) -> Self {
        Self {
            x: rect.x(),
            y: rect.y(),
            width: rect.width(),
            height: rect.height(),
        }
    }
}
