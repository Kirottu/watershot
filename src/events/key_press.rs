use std::{
    cell::RefCell,
    fs::File,
    io::{self, Cursor, Write},
    rc::Rc,
};

use gdk::keys::constants;
use gtk::prelude::*;

use crate::{types::*, RUNTIME_DATA};

pub fn key_press_event(
    _window: &gtk::ApplicationWindow,
    event: &gdk::EventKey,
    image: Rc<RefCell<image::DynamicImage>>,
) -> Inhibit {
    match event.keyval() {
        constants::Escape => {
            RUNTIME_DATA.with(|runtime_data| {
                for (window, _) in &runtime_data.borrow().windows {
                    window.close();
                }
            });
            Inhibit(false)
        }
        constants::Tab => {
            RUNTIME_DATA.with(|runtime_data| {
                let mut runtime_data = runtime_data.borrow_mut();
                match runtime_data.selection {
                    Selection::Rectangle(_) => runtime_data.selection = Selection::Display(None),
                    Selection::Display(_) => runtime_data.selection = Selection::Rectangle(None),
                }

                for (_, window_info) in &runtime_data.windows {
                    window_info.selection_overlay.queue_draw();
                }
            });
            Inhibit(false)
        }
        constants::Return => RUNTIME_DATA.with(|runtime_data| {
            let runtime_data = runtime_data.borrow();
            let mut rect = match &runtime_data.selection {
                Selection::Rectangle(Some(selection)) => selection.extents.to_rect(),
                Selection::Display(Some(selection)) => runtime_data.windows[&selection.window]
                    .monitor
                    .geometry()
                    .into(),
                _ => return Inhibit(false),
            };
            for (window, _) in &runtime_data.windows {
                window.close();
            }
            // Convert the coordinate space to one compatible with the source image
            rect.x -= runtime_data.area_rect.unwrap().x;
            rect.y -= runtime_data.area_rect.unwrap().y;

            // Crop the selection area
            let image = image.borrow_mut().crop(
                rect.x as u32,
                rect.y as u32,
                rect.width as u32,
                rect.height as u32,
            );

            // Write it to the buffer using a format
            let mut buf = Cursor::new(Vec::new());
            image
                .write_to(&mut buf, image::ImageOutputFormat::Png)
                .unwrap();

            let buf = buf.into_inner();

            // Write the final image to outputs based on args
            if runtime_data.args.stdout {
                io::stdout().write_all(&buf).unwrap();
            }
            if let Some(path) = &runtime_data.args.path {
                let mut file = File::create(format!(
                    "{}/WaterShot_{}.png",
                    path,
                    chrono::Local::now().format("%d-%m-%Y_%H:%M")
                ))
                .unwrap();
                file.write_all(&buf).unwrap();
            }
            Inhibit(true)
        }),
        _ => Inhibit(false),
    }
}
