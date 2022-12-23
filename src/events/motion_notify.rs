use gtk::prelude::*;

use crate::{traits::*, types::*, RUNTIME_DATA};

pub fn motion_notify_event(window: &gtk::ApplicationWindow, event: &gdk::EventMotion) -> Inhibit {
    RUNTIME_DATA.with(|runtime_data| {
        let mut runtime_data = runtime_data.borrow_mut();

        // Only handle motion if rectangular selection is active
        if let Selection::Rectangle(Some(selection)) = &runtime_data.selection {
            if selection.active {
                // Queue each one of the drawing layers for redrawing to update them
                for (_, window_info) in &runtime_data.windows {
                    window_info.selection_overlay.queue_draw();
                }

                // Convert it to the global coordinate space
                let pos: (i32, i32) = event
                    .position()
                    .to_global(&runtime_data.windows[window].monitor);

                // Get a mutable reference here, has to be done like this to avoid needlessly redrawing
                // Drawing areas
                let area = runtime_data.area_rect;
                let selection = match &mut runtime_data.selection {
                    Selection::Rectangle(Some(selection)) => selection,
                    _ => unreachable!(),
                };

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

                            selection.extents =
                                extents.to_rect_clamped(&area.unwrap()).to_extents();
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
