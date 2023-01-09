use smithay_client_toolkit::{
    delegate_keyboard,
    reexports::client::{
        protocol::{wl_keyboard, wl_surface},
        Connection, QueueHandle,
    },
    seat::keyboard::{keysyms, KeyEvent, KeyboardHandler, Modifiers},
};

use crate::{
    runtime_data::RuntimeData,
    types::{ExitState, Selection},
};

delegate_keyboard!(RuntimeData);

impl KeyboardHandler for RuntimeData {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _: &[u32],
    ) {
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
    ) {
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        match event.keysym {
            // Exit without copying/saving
            keysyms::XKB_KEY_Escape => self.exit = ExitState::ExitOnly,
            // Switch selection mode
            keysyms::XKB_KEY_Tab => {
                for monitor in &mut self.monitors {
                    monitor.damage.push(monitor.rect);
                }

                match &self.selection {
                    Selection::Rectangle(_) => self.selection = Selection::Display(None),
                    Selection::Display(_) => self.selection = Selection::Rectangle(None),
                }
            }
            // Exit with save if a valid selection exists
            keysyms::XKB_KEY_Return => match &self.selection {
                Selection::Rectangle(Some(selection)) => {
                    let mut rect = selection.extents.to_rect();
                    // Alter coordinate space so the rect can be used to crop from the original image
                    rect.x -= self.area.x;
                    rect.y -= self.area.y;

                    self.exit = ExitState::ExitWithSelection(rect)
                }
                Selection::Display(Some(selection)) => {
                    let monitor = self
                        .monitors
                        .iter()
                        .find(|monitor| *monitor.layer.wl_surface() == selection.surface)
                        .unwrap();

                    let mut rect = monitor.rect;

                    rect.x -= self.area.x;
                    rect.y -= self.area.y;

                    self.exit = ExitState::ExitWithSelection(rect)
                }
                _ => (),
            },
            _ => (),
        }
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        println!("Key release: {:?}", event);
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
    ) {
        println!("Update modifiers: {:?}", modifiers);
    }
}
