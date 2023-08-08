use crate::types::Rect;

use self::clone::DescribesWindowClone;

mod clone;
pub mod hyprland;
pub mod search;

pub trait CompositorBackend {
    fn get_all_windows(&self) -> Vec<Box<dyn DescribesWindow>>;
    fn get_focused(&self) -> Option<Box<dyn DescribesWindow>>;
}

pub trait InitializeBackend {
    fn try_new() -> Result<Box<dyn CompositorBackend>, CompositorNotAvailable>;
}

pub trait DescribesWindow: DescribesWindowClone {
    fn get_window_rect(&self) -> Rect<i32>;
    fn initial_title(&self) -> &str;
    fn title(&self) -> &str;
    fn initial_class(&self) -> &str;
    fn class(&self) -> &str;
}

pub trait GetsMouse {
    fn get_mouse_position() -> (i32, i32);
}

pub type MouseGetter = hyprland::HyprMouseGetter;

pub trait FindWindowExt {
    fn find_by_position(&self, x: i32, y: i32) -> Option<&Box<dyn DescribesWindow>>;
    fn find_by_search_param(
        &self,
        param: search::WindowSearchParam,
    ) -> Option<&Box<dyn DescribesWindow>>;
}

impl FindWindowExt for Vec<Box<dyn DescribesWindow>> {
    fn find_by_position(&self, x: i32, y: i32) -> Option<&Box<dyn DescribesWindow>> {
        self.iter()
            .find(|window| window.get_window_rect().contains_point(x, y))
    }

    fn find_by_search_param(
        &self,
        param: search::WindowSearchParam,
    ) -> Option<&Box<dyn DescribesWindow>> {
        use search::WindowSearchAttribute::*;

        self.iter().find(|window| {
            let attr_value = match param.attribute {
                InitialTitle => window.initial_title(),
                Title => window.title(),
                InitialClass => window.initial_class(),
                Class => window.class(),
            };

            param.value.is_match(attr_value)
        })
    }
}

pub enum CompositorNotAvailable {
    NotInstalled,
    NotRunning,
}
