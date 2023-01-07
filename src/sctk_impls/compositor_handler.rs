use smithay_client_toolkit::{
    compositor::CompositorHandler,
    delegate_compositor,
    reexports::client::{protocol::wl_surface, Connection, QueueHandle},
};

use crate::{runtime_data::RuntimeData, types::MonitorIdentification};

delegate_compositor!(RuntimeData);

impl CompositorHandler for RuntimeData {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.draw(MonitorIdentification::Surface(surface.clone()), qh);
    }
}
