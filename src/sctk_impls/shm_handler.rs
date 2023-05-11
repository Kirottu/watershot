use smithay_client_toolkit::{
    delegate_shm,
    shm::{Shm, ShmHandler},
};

use crate::runtime_data::RuntimeData;

delegate_shm!(RuntimeData);

impl ShmHandler for RuntimeData {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}
