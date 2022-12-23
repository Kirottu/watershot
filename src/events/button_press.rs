use crate::{handles, traits::*, types::*, RUNTIME_DATA};
use gtk::prelude::*;

/// Handle mouse button presses to alter the selection
pub fn button_press_event(window: &gtk::ApplicationWindow, event: &gdk::EventButton) -> Inhibit {
    RUNTIME_DATA.with(|runtime_data| {
        if event.event_type() == gdk::EventType::ButtonPress {
            let mut runtime_data = runtime_data.borrow_mut();
            for (_, window_info) in &runtime_data.windows {
                window_info.selection_overlay.queue_draw();
            }

            // Transform event position to a global position
            let transformed_pos = event
                .position()
                .to_global(&runtime_data.windows[window].monitor);

            // Copy this locally due to the later mutable borrow
            let radius = runtime_data.config.handle_radius;

            match &mut runtime_data.selection {
                Selection::Rectangle(selection) => {
                    if let Some(selection) = selection {
                        // Check for the drag handles
                        for (x, y, modifier) in handles!(selection.extents) {
                            if transformed_pos.distance_to((*x as f64, *y as f64)) <= radius as f64
                            {
                                selection.modifier = Some(*modifier);
                                selection.active = true;
                                return;
                            }
                        }
                        // Check if the cursor is inside the selected area
                        if selection
                            .extents
                            .to_rect()
                            .contains(transformed_pos.0, transformed_pos.1)
                        {
                            selection.modifier = Some(SelectionModifier::Center(
                                transformed_pos.0,
                                transformed_pos.1,
                                selection.extents,
                            ));
                            selection.active = true;
                            return;
                        }
                    }
                    // If no selection exists or click is outside handles & selection, create a new selection
                    runtime_data.selection = Selection::Rectangle(Some(RectangleSelection::new(
                        transformed_pos.0,
                        transformed_pos.1,
                    )));
                }
                Selection::Display(selection) => {
                    // Set the active display
                    *selection = Some(DisplaySelection::new(window.clone()));
                }
            }
        }
    });

    // Other events need to be inhibited so the button press event is not duplicated.
    // It's weird but it works
    Inhibit(true)
}
