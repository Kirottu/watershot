use smithay_client_toolkit::{
    delegate_layer,
    reexports::client::{Connection, QueueHandle},
    shell::wlr_layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
};

use crate::{
    rendering::{MonSpecificRendering, Renderer},
    runtime_data::RuntimeData,
    types::MonitorIdentification,
};

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

        log::info!("{:?}", _configure);

        let monitor = self
            .monitors
            .iter()
            .find(|window| window.layer == *layer)
            .unwrap();

        let cap = monitor.surface.get_capabilities(&self.adapter);

        if self.renderer.is_none() {
            self.renderer = Some(Renderer::new(&self.device, &self.config, cap.formats[0]));
        }

        monitor.surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: cap.formats[0],
                width: (monitor.rect.width * monitor.output_info.scale_factor) as u32,
                height: (monitor.rect.height * monitor.output_info.scale_factor) as u32,
                present_mode: wgpu::PresentMode::Mailbox,
                alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                view_formats: vec![cap.formats[0]],
            },
        );

        let mon_rendering = MonSpecificRendering::new(
            &monitor.rect,
            &monitor.output_info,
            cap.formats[0],
            monitor.image.to_rgba8(),
            self,
        );

        // Reborrow mutably to set the renderer
        let monitor = self
            .monitors
            .iter_mut()
            .find(|window| window.layer == *layer)
            .unwrap();

        monitor.rendering = Some(mon_rendering);

        log::info!("{:?}", cap.formats);

        self.draw(MonitorIdentification::Layer(layer.clone()), qh);
    }
}
