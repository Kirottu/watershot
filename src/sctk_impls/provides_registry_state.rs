use smithay_client_toolkit::{
    delegate_registry,
    output::OutputState,
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::SeatState,
};

use crate::runtime_data::RuntimeData;

delegate_registry!(RuntimeData);

impl ProvidesRegistryState for RuntimeData {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}
