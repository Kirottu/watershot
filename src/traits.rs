use fontdue::{
    layout::{CoordinateSystem, Layout, TextStyle},
    Font,
};
use raqote::{DrawOptions, DrawTarget, Image, SolidSource};

use crate::types::{Color, Extents, Rect};

pub trait ToLocal<T> {
    fn to_local(&self, rect: &Rect) -> T;
}

pub trait ToGlobal<T> {
    fn to_global(&self, rect: &Rect) -> T;
}

pub trait DistanceTo<T> {
    fn distance_to(&self, other: &(T, T)) -> T;
}

pub trait DrawText {
    fn draw_text(&mut self, x: f32, y: f32, font: &[Font], text: &str, size: f32, color: Color);
}

pub trait Crop {
    fn crop(&self, rect: &Rect) -> Vec<u32>;
}

impl<'a> Crop for Image<'a> {
    fn crop(&self, rect: &Rect) -> Vec<u32> {
        assert!(rect.x + rect.width <= self.width && rect.y + rect.height <= self.height);
        assert!(rect.x + rect.width >= 0 && rect.y + rect.height >= 0);
        self.data
            .chunks_exact(self.width as usize)
            .skip(rect.y as usize)
            .enumerate()
            .take_while(|(i, _)| *i < (rect.y + rect.height) as usize)
            .flat_map(|(_, data)| &data[rect.x as usize..(rect.x + rect.width) as usize])
            .copied()
            .collect::<Vec<u32>>()
    }
}

impl DrawText for DrawTarget<&mut [u32]> {
    fn draw_text(&mut self, x: f32, y: f32, font: &[Font], text: &str, size: f32, color: Color) {
        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.append(font, &TextStyle::new(text, size, 0));

        let last = layout.glyphs().last().unwrap();

        let total_width = last.width as f32 + last.x;
        let total_height = layout.height();

        for glyph in layout.glyphs() {
            let (_, buf) = font[0].rasterize_config(glyph.key);

            let data = buf
                .into_iter()
                .map(|coverage| {
                    SolidSource::from_unpremultiplied_argb(
                        (coverage as u32 * color.a as u32 / 255) as u8,
                        color.r,
                        color.g,
                        color.b,
                    )
                    .to_u32()
                })
                .collect::<Vec<_>>();

            self.draw_image_at(
                x + glyph.x - total_width / 2.0,
                y + glyph.y - total_height / 2.0,
                &Image {
                    width: glyph.width as i32,
                    height: glyph.height as i32,
                    data: &data,
                },
                &DrawOptions::default(),
            );
        }
    }
}

impl ToLocal<Extents> for Extents {
    fn to_local(&self, rect: &Rect) -> Extents {
        Self {
            start_x: self.start_x - rect.x,
            start_y: self.start_y - rect.y,
            end_x: self.end_x - rect.x,
            end_y: self.end_y - rect.y,
        }
    }
}

impl ToLocal<Rect> for Rect {
    fn to_local(&self, rect: &Rect) -> Rect {
        Rect::new(self.x - rect.x, self.y - rect.y, self.width, self.height)
    }
}

impl DistanceTo<i32> for (i32, i32) {
    fn distance_to(&self, other: &(i32, i32)) -> i32 {
        let x = (other.0 - self.0) as f64;
        let y = (other.1 - self.1) as f64;
        f64::sqrt(x * x + y * y) as i32
    }
}

impl ToGlobal<(i32, i32)> for (f64, f64) {
    fn to_global(&self, rect: &Rect) -> (i32, i32) {
        (self.0 as i32 + rect.x, self.1 as i32 + rect.y)
    }
}
