use gtk::prelude::*;

use crate::{types::*, RUNTIME_DATA};

pub fn button_release_event(_window: &gtk::ApplicationWindow, event: &gdk::EventButton) -> Inhibit {
    RUNTIME_DATA.with(|runtime_data| {
        // Stop modifying the selection when the button is released
        if event.event_type() == gdk::EventType::ButtonRelease {
            if let Selection::Rectangle(Some(selection)) = &mut runtime_data.borrow_mut().selection
            {
                selection.active = false;
                selection.modifier = None;
            }
        }
    });

    Inhibit(true)
}
