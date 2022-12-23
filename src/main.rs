use std::{
    cell::RefCell,
    collections::HashMap,
    process::{self, Command},
    rc::Rc,
};

use gdk::prelude::*;
use gtk::prelude::*;
use types::*;

mod macros;
mod traits;
mod types;
mod events {
    pub mod button_press;
    pub mod button_release;
    pub mod draw;
    pub mod key_press;
    pub mod motion_notify;
}

// Thread local due to RefCell, which is caused by GTK being single threaded
thread_local! {
    // The main data operated on at runtime
    pub static RUNTIME_DATA: RefCell<RuntimeData> = RefCell::new(RuntimeData {
        selection: Selection::Rectangle(None),
        args: Args::default(),
        area_rect: Rect::default(),
        config: Config::load().unwrap_or_default(),
        windows: HashMap::new(),
    });
}

fn main() {
    let app = gtk::Application::new(Some("com.kirottu.WaterShot"), Default::default());

    // Launch options are handled by GTK
    app.add_main_option(
        "stdout",
        glib::Char('s' as i8),
        glib::OptionFlags::IN_MAIN,
        glib::OptionArg::None,
        "Output final image to stdout",
        None,
    );

    app.add_main_option(
        "path",
        glib::Char('p' as i8),
        glib::OptionFlags::IN_MAIN,
        glib::OptionArg::String,
        "Save final image to directory",
        None,
    );

    app.add_main_option(
        "grim",
        glib::Char('g' as i8),
        glib::OptionFlags::IN_MAIN,
        glib::OptionArg::String,
        "Path to grim",
        None,
    );

    app.connect_activate(move |app| {
        activate(app);
    });

    // Parse the launch options and place them in the proper variables
    app.connect_handle_local_options(|_app, variant_dict| {
        RUNTIME_DATA.with(|runtime_data| {
            let mut runtime_data = runtime_data.borrow_mut();
            runtime_data.args.stdout = variant_dict.contains("stdout");
            runtime_data.args.path = variant_dict.lookup::<String>("path").unwrap();
            runtime_data.args.grim = variant_dict.lookup::<String>("grim").unwrap();
        });
        -1 // GTK magic number to continue normally
    });

    app.run();
}

fn activate(app: &gtk::Application) {
    let display = gdk::Display::default().unwrap();

    let bytes = RUNTIME_DATA.with(|runtime_data| {
        match Command::new(
            runtime_data
                .borrow()
                .args
                .grim
                .as_ref()
                .unwrap_or(&"grim".to_string()),
        )
        .arg("-")
        .output()
        {
            Ok(output) => output.stdout,
            Err(why) => {
                eprintln!("Failed to run grim command: {}", why);
                process::exit(1);
            }
        }
    });

    let image = Rc::new(RefCell::new(image::load_from_memory(&bytes).unwrap()));
    for i in 0..display.n_monitors() {
        let monitor = display.monitor(i).unwrap();
        let window = gtk::ApplicationWindow::new(app);

        // Init gtk-layer-shell with all the parameters.
        // -1 is used for exclusive zone to make it disregard everything else
        gtk_layer_shell::init_for_window(&window);
        gtk_layer_shell::set_layer(&window, gtk_layer_shell::Layer::Overlay);
        gtk_layer_shell::set_monitor(&window, &monitor);
        gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Top, true);
        gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Bottom, true);
        gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Left, true);
        gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Right, true);
        gtk_layer_shell::set_keyboard_mode(&window, gtk_layer_shell::KeyboardMode::OnDemand);
        gtk_layer_shell::set_exclusive_zone(&window, -1);

        window.set_events(gdk::EventMask::POINTER_MOTION_MASK);

        let rect = monitor.geometry();

        // Crop the image appropriate for the current monitor
        let window_image = image.borrow_mut().crop(
            rect.x() as u32,
            rect.y() as u32,
            rect.width() as u32,
            rect.height() as u32,
        );

        // Create a Pixbuf from it for using in the background widget
        let pixbuf = gdk_pixbuf::Pixbuf::from_bytes(
            &gdk::glib::Bytes::from_owned(window_image.into_bytes()),
            gdk_pixbuf::Colorspace::Rgb,
            true,
            8,
            rect.width() as i32,
            rect.height() as i32,
            gdk_pixbuf::Pixbuf::calculate_rowstride(
                gdk_pixbuf::Colorspace::Rgb,
                true,
                8,
                rect.width(),
                rect.height(),
            ),
        );

        // Create the widgets
        let image_widget = gtk::Image::from_pixbuf(Some(&pixbuf));
        let overlay = gtk::Overlay::builder().child(&image_widget).build();
        let selection_overlay = gtk::DrawingArea::builder().build();

        let image_clone = image.clone();

        // All of the different events that need to be handled
        selection_overlay
            .connect_draw(|selection_overlay, ctx| events::draw::draw(selection_overlay, ctx));
        window.connect_button_press_event(|window, event| {
            events::button_press::button_press_event(window, event)
        });
        window.connect_button_release_event(|window, event| {
            events::button_release::button_release_event(window, event)
        });
        window.connect_motion_notify_event(|window, event| {
            events::motion_notify::motion_notify_event(window, event)
        });
        window.connect_key_press_event(move |window, event| {
            events::key_press::key_press_event(window, event, image_clone.clone())
        });

        // Clone some variables and insert them into the runtime data for later use
        let selection_overlay_clone = selection_overlay.clone();
        let window_clone = window.clone();

        RUNTIME_DATA.with(|runtime_data| {
            let mut runtime_data = runtime_data.borrow_mut();

            runtime_data.area_rect.extend(&rect.into());
            runtime_data.windows.insert(
                window_clone.clone(),
                WindowInfo {
                    selection_overlay: selection_overlay_clone,
                    monitor,
                },
            );
        });

        // Add widgets together and show them
        overlay.add_overlay(&selection_overlay);
        window.add(&overlay);
        window.show_all();
    }
}
