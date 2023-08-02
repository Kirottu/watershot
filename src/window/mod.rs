use crate::types::Rect;

#[cfg(feature = "hyprland-window-selection")]
pub mod hyprland;
pub mod search;

pub trait DescribesWindow: Sized {
    fn get_all_windows() -> Vec<Self>;
    fn get_focused() -> Option<Self>;

    fn get_window_rect(&self) -> Rect<i32>;
    fn initial_title(&self) -> &str;
    fn title(&self) -> &str;
    fn initial_class(&self) -> &str;
    fn class(&self) -> &str;
}

pub trait GetsMouse {
    fn get_mouse_position() -> (i32, i32);
}

#[cfg(feature = "hyprland-window-selection")]
pub type WindowDescriptor = hyprland::HyprWindowDescriptor;

#[cfg(feature = "hyprland-window-selection")]
pub type MouseGetter = hyprland::HyprMouseGetter;

pub trait FindWindow: Sized {
    fn find_by_position(&self, x: i32, y: i32) -> Option<&WindowDescriptor>;
    fn find_by_search_param(&self, param: search::WindowSearchParam) -> Option<&WindowDescriptor>;
}

impl FindWindow for Vec<WindowDescriptor> {
    fn find_by_position(&self, x: i32, y: i32) -> Option<&WindowDescriptor> {
        self.iter()
            .find(|window| window.get_window_rect().contains_point(x, y))
    }

    fn find_by_search_param(&self, param: search::WindowSearchParam) -> Option<&WindowDescriptor> {
        self.iter().find(|window| {
            let attr_value = match param.attribute {
                search::WindowSearchAttribute::InitialTitle => window.initial_title(),
                search::WindowSearchAttribute::Title => window.title(),
                search::WindowSearchAttribute::InitialClass => window.initial_class(),
                search::WindowSearchAttribute::Class => window.class(),
            };

            param.value.is_match(attr_value)
        })
    }
}
