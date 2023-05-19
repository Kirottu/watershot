use log::info;
use smithay_client_toolkit::{
    delegate_seat,
    reexports::client::{protocol::wl_seat, Connection, QueueHandle},
    seat::{pointer::ThemeSpec, Capability, SeatHandler, SeatState},
};

use crate::runtime_data::RuntimeData;

delegate_seat!(RuntimeData);

impl SeatHandler for RuntimeData {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            info!("Set keyboard capability");
            let keyboard = self
                .seat_state
                .get_keyboard(qh, &seat, None)
                .expect("Failed to create keyboard");
            self.keyboard = Some(keyboard);
        }

        if capability == Capability::Pointer
            && self.pointer.is_none()
            && self.themed_pointer.is_none()
        {
            info!("Set pointer capability");

            let themed_pointer = self
                .seat_state
                .get_pointer_with_theme(qh, &seat, ThemeSpec::default())
                .expect("Failed to create themed pointer");
            self.pointer = Some(themed_pointer.pointer().clone());
            self.themed_pointer = Some(themed_pointer);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_some() {
            info!("Unset keyboard capability");
            self.keyboard.take().unwrap().release();
        }

        if capability == Capability::Pointer && self.pointer.is_some() {
            info!("Unset pointer capability");
            self.pointer.take().unwrap().release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}
