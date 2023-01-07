use smithay_client_toolkit::{
    delegate_shm,
    shm::{ShmHandler, ShmState},
};

use crate::runtime_data::RuntimeData;

delegate_shm!(RuntimeData);

impl ShmHandler for RuntimeData {
    fn shm_state(&mut self) -> &mut ShmState {
        &mut self.shm_state
    }
}
