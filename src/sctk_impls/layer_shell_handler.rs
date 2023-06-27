use smithay_client_toolkit::{
    delegate_layer,
    reexports::client::{Connection, QueueHandle},
    shell::wlr_layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
};

use crate::{runtime_data::RuntimeData, types::MonitorIdentification};

delegate_layer!(RuntimeData);

impl LayerShellHandler for RuntimeData {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {}

    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        _configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let _ = self.themed_pointer.as_ref().unwrap().set_cursor(
            conn,
            "crosshair",
            self.shm_state.wl_shm(),
            &self.pointer_surface,
            1,
        );

        let monitor = self
            .monitors
            .iter_mut()
            .find(|window| window.layer == *layer)
            .unwrap();
        let cap = monitor.surface.get_capabilities(&self.adapter);
        monitor.surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: cap.formats[0],
                width: monitor.rect.width as u32,
                height: monitor.rect.height as u32,
                present_mode: wgpu::PresentMode::Mailbox,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![cap.formats[0]],
            },
        );

        log::info!("{:?}", cap.formats[0]);

        self.draw(MonitorIdentification::Layer(layer.clone()), qh);
    }
}
