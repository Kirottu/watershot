use smithay_client_toolkit::{
    delegate_pointer,
    reexports::client::{protocol::wl_pointer, Connection, QueueHandle},
    seat::pointer::{PointerEvent, PointerEventKind, PointerHandler},
};

use crate::{
    handles,
    runtime_data::RuntimeData,
    traits::{DistanceTo, ToGlobal},
    types::{DisplaySelection, RectangleSelection, Selection, SelectionModifier},
};

delegate_pointer!(RuntimeData);

impl PointerHandler for RuntimeData {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        use PointerEventKind::*;
        for event in events {
            let layer = self
                .monitors
                .iter()
                .find(|layer| *layer.layer.wl_surface() == event.surface)
                .unwrap();
            let global_pos = event.position.to_global(&layer.rect);

            match event.kind {
                Enter { .. } => {
                    println!("Pointer entered @{:?}", event.position);
                }
                Leave { .. } => {
                    println!("Pointer left");
                }
                Motion { .. } => {
                    if let Selection::Rectangle(Some(selection)) = &mut self.selection {
                        if selection.active {
                            match selection.modifier {
                                // Handle selection modifiers, AKA the drag handles and moving it from the center
                                Some(modifier) => match modifier {
                                    SelectionModifier::Left => {
                                        selection.extents.start_x = global_pos.0
                                    }
                                    SelectionModifier::Right => {
                                        selection.extents.end_x = global_pos.0
                                    }
                                    SelectionModifier::Top => {
                                        selection.extents.start_y = global_pos.1
                                    }
                                    SelectionModifier::Bottom => {
                                        selection.extents.end_y = global_pos.1
                                    }
                                    SelectionModifier::TopRight => {
                                        selection.extents.end_x = global_pos.0;
                                        selection.extents.start_y = global_pos.1;
                                    }
                                    SelectionModifier::BottomRight => {
                                        selection.extents.end_x = global_pos.0;
                                        selection.extents.end_y = global_pos.1;
                                    }
                                    SelectionModifier::BottomLeft => {
                                        selection.extents.start_x = global_pos.0;
                                        selection.extents.end_y = global_pos.1;
                                    }
                                    SelectionModifier::TopLeft => {
                                        selection.extents.start_x = global_pos.0;
                                        selection.extents.start_y = global_pos.1;
                                    }
                                    SelectionModifier::Center(x, y, mut extents) => {
                                        extents.start_x -= x - global_pos.0;
                                        extents.start_y -= y - global_pos.1;
                                        extents.end_x -= x - global_pos.0;
                                        extents.end_y -= y - global_pos.1;

                                        selection.extents =
                                            extents.to_rect_clamped(&self.area).to_extents();
                                    }
                                },
                                None => {
                                    selection.extents.end_x = global_pos.0;
                                    selection.extents.end_y = global_pos.1;
                                }
                            }
                            for monitor in &mut self.monitors {
                                // Extra padding is added to make sure no artifacts remain on displays
                                if monitor
                                    .rect
                                    .intersects(&selection.extents.to_rect().padded(20))
                                {
                                    monitor.draw = true;
                                }
                            }
                        }
                    }
                }
                Press { button, .. } => {
                    println!("Press {:x} @ {:?}", button, event.position);
                    // Redraw all the monitor layers
                    for monitor in &mut self.monitors {
                        monitor.draw = true;
                    }

                    match &mut self.selection {
                        Selection::Rectangle(selection) => {
                            if let Some(selection) = selection {
                                for (x, y, modifier) in handles!(selection.extents) {
                                    if global_pos.distance_to(&(*x, *y))
                                        <= self.config.handle_radius
                                    {
                                        selection.modifier = Some(*modifier);
                                        selection.active = true;
                                        return;
                                    }
                                }
                                if selection
                                    .extents
                                    .to_rect()
                                    .contains(global_pos.0, global_pos.1)
                                {
                                    selection.modifier = Some(SelectionModifier::Center(
                                        global_pos.0,
                                        global_pos.1,
                                        selection.extents,
                                    ));
                                    selection.active = true;
                                    return;
                                }
                            }

                            self.selection = Selection::Rectangle(Some(RectangleSelection::new(
                                global_pos.0,
                                global_pos.1,
                            )));
                        }
                        Selection::Display(_) => {
                            self.selection = Selection::Display(Some(DisplaySelection::new(
                                event.surface.clone(),
                            )));
                        }
                    }
                }
                Release { button, .. } => {
                    println!("Release {:x} @ {:?}", button, event.position);

                    if let Selection::Rectangle(Some(selection)) = &mut self.selection {
                        selection.active = false;
                    }
                }
                Axis {
                    horizontal,
                    vertical,
                    ..
                } => {
                    println!("Scroll H:{:?}, V:{:?}", horizontal, vertical);
                }
            }
        }
    }
}
