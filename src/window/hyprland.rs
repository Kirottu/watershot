use hyprland::{
    data::{Clients, WorkspaceBasic, Workspaces},
    shared::{HyprData, HyprDataVec},
};

use crate::types::Rect;

use super::DescribesWindow;

#[derive(Debug, Clone)]
pub struct HyprWindowDescriptor {
    initial_title: String,
    title: String,
    initial_class: String,
    class: String,
    rect: Rect<i32>,
    workspace: HyprWorkspaceDescriptor,
    monitor: HyprMonitorDescriptor,
}

#[derive(Debug, Clone)]
pub struct HyprWorkspaceDescriptor(pub WorkspaceBasic);

#[derive(Debug, Clone)]
pub struct HyprMonitorDescriptor(i16);

impl DescribesWindow for HyprWindowDescriptor {
    fn get_window_rect(&self) -> Rect<i32> {
        self.rect
    }

    fn get_all_windows() -> Vec<Self> {
        Clients::get()
            .unwrap()
            .iter()
            // Filter out special workspaces. Is it possible to check if they're toggled on?
            .filter(|client| client.workspace.id >= 0)
            .map(|client| Self {
                initial_title: client.initial_title.clone(),
                title: client.title.clone(),
                initial_class: client.initial_class.clone(),
                class: client.class.clone(),
                rect: Rect {
                    x: client.at.0 as i32,
                    y: client.at.1 as i32,
                    width: client.size.0 as i32,
                    height: client.size.1 as i32,
                },
                workspace: HyprWorkspaceDescriptor(client.workspace.clone()),
                monitor: HyprMonitorDescriptor(client.monitor),
            })
            .collect()
    }
}
