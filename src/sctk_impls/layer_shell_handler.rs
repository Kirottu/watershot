use smithay_client_toolkit::{
    delegate_layer,
    reexports::client::{Connection, QueueHandle},
    shell::layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
};

use crate::{runtime_data::RuntimeData, types::MonitorIdentification};

delegate_layer!(RuntimeData);

impl LayerShellHandler for RuntimeData {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {}

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        _configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        self.draw(MonitorIdentification::Layer(layer.clone()), qh);
    }
}
