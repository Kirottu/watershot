use gtk::prelude::*;

use crate::{traits::*, types::*, RUNTIME_DATA};

pub fn motion_notify_event(_window: &gtk::ApplicationWindow, event: &gdk::EventMotion) -> Inhibit {
    RUNTIME_DATA.with(|runtime_data| {
        let mut runtime_data = runtime_data.borrow_mut();

        // Using device data to figure out the position of the cursor and the correct monitor.
        // Due to wacky GTK stuff, the window that receives the event when the mouse button is
        // held down is not the one actually under it.
        // TODO: This is dumb.
        let device = match event.device() {
            Some(dev) => dev,
            None => {
                eprintln!("Unable to get device!");
                return;
            }
        };
        let dev_pos = device.position();

        let (window, _, _) = device.window_at_position_double();
        let window = match window {
            Some(window) => window,
            None => {
                eprintln!("Unable to get window!");
                return;
            }
        };

        let monitor = match window.display().monitor_at_window(&window) {
            Some(monitor) => monitor,
            None => {
                eprintln!("Unable to get monitor!");
                return;
            }
        };

        if let Selection::Rectangle(Some(selection)) = &runtime_data.selection {
            if selection.active {
                // Queue each one of the drawing layers for redrawing to update them
                for (_, window_info) in &runtime_data.windows {
                    window_info.selection_overlay.queue_draw();
                }

                // Get a mutable reference here, has to be done like this to avoid needlessly redrawing
                // Drawing areas
                let area = runtime_data.area_rect;
                let selection = match &mut runtime_data.selection {
                    Selection::Rectangle(Some(selection)) => selection,
                    _ => unreachable!(),
                };

                // Convert it to the global coordinate space
                let pos: (i32, i32) = (dev_pos.1, dev_pos.2).to_global(&monitor);

                match selection.modifier {
                    // Handle selection modifiers, AKA the drag handles and moving it from the center
                    Some(modifier) => match modifier {
                        SelectionModifier::Left => selection.extents.start_x = pos.0,
                        SelectionModifier::Right => selection.extents.end_x = pos.0,
                        SelectionModifier::Top => selection.extents.start_y = pos.1,
                        SelectionModifier::Bottom => selection.extents.end_y = pos.1,
                        SelectionModifier::TopRight => {
                            selection.extents.end_x = pos.0;
                            selection.extents.start_y = pos.1;
                        }
                        SelectionModifier::BottomRight => {
                            selection.extents.end_x = pos.0;
                            selection.extents.end_y = pos.1;
                        }
                        SelectionModifier::BottomLeft => {
                            selection.extents.start_x = pos.0;
                            selection.extents.end_y = pos.1;
                        }
                        SelectionModifier::TopLeft => {
                            selection.extents.start_x = pos.0;
                            selection.extents.start_y = pos.1;
                        }
                        SelectionModifier::Center(x, y, mut extents) => {
                            extents.start_x -= x - pos.0;
                            extents.start_y -= y - pos.1;
                            extents.end_x -= x - pos.0;
                            extents.end_y -= y - pos.1;

                            selection.extents = extents.to_rect_clamped(&area).to_extents();
                        }
                    },
                    None => {
                        selection.extents.end_x = pos.0;
                        selection.extents.end_y = pos.1;
                    }
                }
            }
        }
    });

    Inhibit(true)
}
