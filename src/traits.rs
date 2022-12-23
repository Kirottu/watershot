use crate::Extents;

// Helper traits
pub trait Transform<T> {
    fn to_global(&self, _monitor: &gdk::Monitor) -> T {
        todo!();
    }
    fn to_local(&self, _monitor: &gdk::Monitor) -> T {
        todo!();
    }
}

impl Transform<(i32, i32)> for (f64, f64) {
    fn to_global(&self, monitor: &gdk::Monitor) -> (i32, i32) {
        (
            self.0 as i32 + monitor.geometry().x(),
            self.1 as i32 + monitor.geometry().y(),
        )
    }
}

impl Transform<(f64, f64)> for (i32, i32) {
    fn to_local(&self, monitor: &gdk::Monitor) -> (f64, f64) {
        (
            self.0 as f64 - monitor.geometry().x() as f64,
            self.1 as f64 - monitor.geometry().y() as f64,
        )
    }
}

impl Transform<(i32, i32)> for (i32, i32) {
    fn to_global(&self, monitor: &gdk::Monitor) -> (i32, i32) {
        (
            self.0 + monitor.geometry().x(),
            self.1 + monitor.geometry().y(),
        )
    }
}

impl Transform<Extents> for Extents {
    fn to_local(&self, monitor: &gdk::Monitor) -> Extents {
        Self {
            start_x: self.start_x - monitor.geometry().x(),
            start_y: self.start_y - monitor.geometry().y(),
            end_x: self.end_x - monitor.geometry().x(),
            end_y: self.end_y - monitor.geometry().y(),
        }
    }
}

pub trait DistanceTo<T> {
    fn distance_to(&self, other: (T, T)) -> T;
}

impl DistanceTo<f64> for (f64, f64) {
    fn distance_to(&self, other: (f64, f64)) -> f64 {
        ((other.0 - self.0) * (other.0 - self.0) + (other.1 - self.1) * (other.1 - self.1)).sqrt()
    }
}

impl DistanceTo<f64> for (i32, i32) {
    fn distance_to(&self, other: (f64, f64)) -> f64 {
        ((other.0 - self.0 as f64) * (other.0 - self.0 as f64)
            + (other.1 - self.1 as f64) * (other.1 - self.1 as f64))
            .sqrt()
    }
}
