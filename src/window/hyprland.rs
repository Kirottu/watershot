use std::collections::HashSet;

use hyprland::{
    data::{Client, Clients, CursorPosition, Monitors, Version},
    shared::{HyprData, HyprDataActiveOptional, WorkspaceId},
};

use crate::types::Rect;

use super::{CompositorBackend, InitializeBackend, WindowDescriptor};

pub struct HyprlandBackend;

impl CompositorBackend for HyprlandBackend {
    fn get_all_windows(&self) -> Vec<WindowDescriptor> {
        // TODO: Sepecial Workspaces don't appear under monitors, therefore
        // windows from specials can't be focused yet.
        let active_workspace_ids: HashSet<WorkspaceId> = Monitors::get()
            .unwrap()
            .iter()
            .map(|monitor| monitor.active_workspace.id)
            .collect();

        let mut windows: Vec<_> = Clients::get()
            .unwrap()
            .filter(|client| active_workspace_ids.contains(&client.workspace.id))
            .map(WindowDescriptor::from)
            .collect();

        windows.reverse();

        windows
    }

    fn get_focused(&self) -> Option<WindowDescriptor> {
        Client::get_active()
            .ok()
            .flatten()
            .map(WindowDescriptor::from)
    }

    fn get_mouse_position(&self) -> (i32, i32) {
        let CursorPosition { x, y } = CursorPosition::get().unwrap();
        (x as i32, y as i32)
    }
}

impl InitializeBackend for HyprlandBackend {
    fn try_new() -> Result<Box<dyn CompositorBackend>, super::CompositorNotAvailable> {
        Version::get()
            .map(|_| Box::new(HyprlandBackend) as Box<dyn CompositorBackend>)
            .map_err(|_| super::CompositorNotAvailable::NotInstalled)
    }
}

impl From<Client> for WindowDescriptor {
    fn from(value: Client) -> Self {
        Self {
            initial_title: value.initial_title,
            title: value.title,
            initial_class: value.initial_class,
            class: value.class,
            rect: Rect {
                x: value.at.0 as i32,
                y: value.at.1 as i32,
                width: value.size.0 as i32,
                height: value.size.1 as i32,
            },
        }
    }
}
