use crate::{traits::Contains, types::Rect};

pub mod hyprland;
pub mod search;

#[derive(Debug, Clone)]
pub struct WindowDescriptor {
    pub initial_title: String,
    pub title: String,
    pub initial_class: String,
    pub class: String,
    pub rect: Rect<i32>,
}

pub trait CompositorBackend {
    fn get_all_windows(&self) -> Vec<WindowDescriptor>;
    fn get_focused(&self) -> Option<WindowDescriptor>;
    fn get_mouse_position(&self) -> (i32, i32);
}

pub trait InitializeBackend {
    fn try_new() -> Result<Box<dyn CompositorBackend>, CompositorNotAvailable>;
}

pub trait FindWindowExt {
    fn find_by_position(&self, position: &(i32, i32)) -> Option<&WindowDescriptor>;
    fn find_by_search_param(&self, param: search::WindowSearchParam) -> Option<&WindowDescriptor>;
}

impl FindWindowExt for Vec<WindowDescriptor> {
    fn find_by_position(&self, position: &(i32, i32)) -> Option<&WindowDescriptor> {
        self.iter().find(|window| window.rect.contains(position))
    }

    fn find_by_search_param(&self, param: search::WindowSearchParam) -> Option<&WindowDescriptor> {
        use search::WindowSearchAttribute::*;

        self.iter().find(|window| {
            let attr_value = match param.attribute {
                InitialTitle => &window.initial_title,
                Title => &window.title,
                InitialClass => &window.initial_class,
                Class => &window.class,
            };

            param.value.is_match(attr_value)
        })
    }
}

pub enum CompositorNotAvailable {
    NotInstalled,
    NotRunning,
}
