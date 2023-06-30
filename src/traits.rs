use crate::types::{Extents, Rect};

pub trait ToLocal<T> {
    fn to_local(&self, rect: &Rect<i32>) -> T;
}

pub trait ToGlobal<T> {
    fn to_global(&self, rect: &Rect<i32>) -> T;
}

pub trait ToRender<T, U> {
    fn to_render(&self, width: U, height: U) -> T;
}

pub trait DistanceTo<T> {
    fn distance_to(&self, other: &(T, T)) -> T;
}

pub trait Contains<T> {
    fn contains(&self, item: &T) -> bool;
}

impl ToRender<Rect<f32>, i32> for Rect<i32> {
    fn to_render(&self, width: i32, height: i32) -> Rect<f32> {
        let width = width as f32;
        let height = height as f32;

        Rect {
            x: (self.x as f32 / width - 0.5) * 2.0,
            y: -(self.y as f32 / height - 0.5) * 2.0,
            width: (self.width as f32 / width) * 2.0,
            height: (self.height as f32 / height) * 2.0,
        }
    }
}

impl ToRender<[f32; 2], i32> for [f32; 2] {
    fn to_render(&self, width: i32, height: i32) -> [f32; 2] {
        [
            (self[0] / width as f32 - 0.5) * 2.0,
            -(self[1] / height as f32 - 0.5) * 2.0,
        ]
    }
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

impl ToLocal<(i32, i32)> for (i32, i32) {
    fn to_local(&self, rect: &Rect<i32>) -> (i32, i32) {
        (self.0 - rect.x, self.1 - rect.y)
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

impl Contains<(i32, i32)> for Rect<i32> {
    fn contains(&self, pos: &(i32, i32)) -> bool {
        pos.0 >= self.x
            && pos.0 <= self.x + self.width
            && pos.1 >= self.y
            && pos.1 <= self.y + self.height
    }
}

impl Contains<Rect<i32>> for Rect<i32> {
    fn contains(&self, other: &Rect<i32>) -> bool {
        self.x <= other.x
            && self.y <= other.y
            && self.x + self.width >= other.x + other.width
            && self.y + self.height >= other.y + other.height
    }
}
