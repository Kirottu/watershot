use crate::types::Rect;

#[cfg(feature = "hyprland-window-selection")]
pub mod hyprland;

pub trait DescribesWindow: Sized {
    fn get_window_rect(&self) -> Rect<i32>;
    fn get_all_windows() -> Vec<Self>;
}

#[cfg(feature = "hyprland-window-selection")]
pub type WindowDescriptor = hyprland::HyprWindowDescriptor;
