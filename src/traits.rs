use fontdue::{
    layout::{CoordinateSystem, Layout, TextStyle},
    Font,
};

use crate::types::{Color, Extents, Rect};

pub trait ToLocal<T> {
    fn to_local(&self, rect: &Rect<i32>) -> T;
}

pub trait ToGlobal<T> {
    fn to_global(&self, rect: &Rect<i32>) -> T;
}

pub trait DistanceTo<T> {
    fn distance_to(&self, other: &(T, T)) -> T;
}

impl ToLocal<Extents> for Extents {
    fn to_local(&self, rect: &Rect<i32>) -> Extents {
        Self {
            start_x: self.start_x - rect.x,
            start_y: self.start_y - rect.y,
            end_x: self.end_x - rect.x,
            end_y: self.end_y - rect.y,
        }
    }
}

impl ToLocal<Rect<i32>> for Rect<i32> {
    fn to_local(&self, rect: &Rect<i32>) -> Rect<i32> {
        Rect::<i32>::new(self.x - rect.x, self.y - rect.y, self.width, self.height)
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
    fn to_global(&self, rect: &Rect<i32>) -> (i32, i32) {
        (self.0 as i32 + rect.x, self.1 as i32 + rect.y)
    }
}
