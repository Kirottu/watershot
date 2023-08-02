use log::info;
use smithay_client_toolkit::{
    delegate_pointer,
    reexports::client::{protocol::wl_pointer, Connection, QueueHandle},
    seat::pointer::{PointerEvent, PointerEventKind, PointerHandler},
};

use crate::{
    handles,
    runtime_data::RuntimeData,
    traits::{Contains, DistanceTo, ToGlobal},
    types::{DisplaySelection, RectangleSelection, Selection, SelectionModifier},
};

#[cfg(feature = "window-selection")]
use crate::window::FindWindow;

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
                .find(|layer| layer.wl_surface == event.surface)
                .unwrap();
            let global_pos = event.position.to_global(&layer.rect);

            match event.kind {
                Enter { .. } => {
                    info!("Pointer entered @{:?}", event.position);
                }
                Leave { .. } => {
                    info!("Pointer left");
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
                        }
                    }
                }
                Press { button, .. } => {
                    info!("Press {:x} @ {:?}", button, event.position);

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
                                if selection.extents.to_rect().contains(&global_pos) {
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
                        #[cfg(feature = "window-selection")]
                        Selection::Window(_) => {
                            self.selection = Selection::Window(
                                self.windows
                                    .find_by_position(global_pos.0, global_pos.1)
                                    .cloned(),
                            );
                        }
                    }
                }
                Release { button, .. } => {
                    info!("Release {:x} @ {:?}", button, event.position);

                    if let Selection::Rectangle(Some(selection)) = &mut self.selection {
                        selection.active = false;
                    }
                }
                Axis {
                    horizontal,
                    vertical,
                    ..
                } => {
                    info!("Scroll H:{:?}, V:{:?}", horizontal, vertical);
                }
            }
        }
    }
}
