#![feature(return_position_impl_trait_in_trait)]

use std::io::{self, Cursor, Write};

use chrono::Local;
use clap::Parser;
use image::{DynamicImage, ImageFormat};
use log::{error, info};
use runtime_data::RuntimeData;
use smithay_client_toolkit::reexports::client::{globals::registry_queue_init, Connection};
use traits::{Contains, ToLocal};
use types::{Args, Config, ExitState, Monitor, Rect, SaveLocation, Selection};
use wl_clipboard_rs::copy;

mod macros;
mod runtime_data;
mod traits;
mod types;

pub mod window;

mod sctk_impls {
    mod compositor_handler;
    mod keyboard_handler;
    mod layer_shell_handler;
    mod output_handler;
    mod pointer_handler;
    mod provides_registry_state;
    mod seat_handler;
    mod shm_handler;
}
mod rendering;

fn main() {
    let args = Args::parse();
    env_logger::init();

    if let Some(image) = gui(&args) {
        // Save the file if an argument for that is present
        if let Some(save_location) = &args.save {
            match save_location {
                SaveLocation::Path { path } => {
                    if let Err(why) = image.save(path) {
                        error!("Error saving image: {}", why);
                    }
                }
                SaveLocation::Directory { path } => {
                    let local = Local::now();
                    if let Err(why) = image.save(
                        local
                            .format(&format!("{}/Watershot_%d-%m-%Y_%H:%M.png", path))
                            .to_string(),
                    ) {
                        error!("Error saving image: {}", why);
                    }
                }
            }
        }

        // Save the selected image into the buffer
        let mut buf = Cursor::new(Vec::new());
        image
            .write_to(&mut buf, ImageFormat::Png)
            .expect("Failed to write image to buffer as PNG");

        let buf = buf.into_inner();

        if args.stdout {
            if let Err(why) = io::stdout().lock().write_all(&buf) {
                error!("Failed to write image content to stdout: {}", why);
            }
        }

        // Fork to serve copy requests
        if args.copy {
            match unsafe { nix::unistd::fork() } {
                Ok(nix::unistd::ForkResult::Parent { .. }) => {
                    info!("Forked to serve copy requests")
                }
                Ok(nix::unistd::ForkResult::Child) => {
                    // Serve copy requests
                    let mut opts = copy::Options::new();
                    opts.foreground(true);
                    opts.copy(
                        copy::Source::Bytes(buf.into_boxed_slice()),
                        copy::MimeType::Autodetect,
                    )
                    .expect("Failed to serve copied image");
                }
                Err(why) => println!("Failed to fork: {}", why),
            }
        }
    }
}

fn gui(args: &Args) -> Option<DynamicImage> {
    let conn = Connection::connect_to_env();
    if conn.is_err() {
        log::error!("Could not connect to the Wayland server, make sure you run watershot within a Wayland session!");
        std::process::exit(1);
    }

    let conn = conn.unwrap();

    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();
    let mut runtime_data = RuntimeData::new(&qh, &globals, args.clone());

    // Fetch the outputs from the compositor
    event_queue.roundtrip(&mut runtime_data).unwrap();
    // Has to be iterated first to get the full area size
    let sizes = runtime_data
        .output_state
        .outputs()
        .map(|output| {
            let info = runtime_data.output_state.info(&output).unwrap();
            let size = info
                .logical_size
                .map(|(w, h)| (w as u32, h as u32))
                .expect("Can't determine monitor size!");
            let pos = info
                .logical_position
                .expect("Can't determine monitor position!");

            let rect = Rect {
                x: pos.0,
                y: pos.1,
                width: size.0 as i32,
                height: size.1 as i32,
            };

            // Extend the area spanning all monitors with the current monitor
            runtime_data.area.extend(&rect);
            (rect, output, info)
        })
        .collect::<Vec<_>>();

    runtime_data.scale_factor = runtime_data.image.width() as f32 / runtime_data.area.width as f32;

    for (rect, output, info) in sizes {
        runtime_data
            .monitors
            .push(Monitor::new(rect, &qh, &conn, output, info, &runtime_data));
    }

    event_queue.roundtrip(&mut runtime_data).unwrap();

    loop {
        event_queue.blocking_dispatch(&mut runtime_data).unwrap();
        match runtime_data.exit {
            ExitState::ExitOnly => return None,
            ExitState::ExitWithSelection(rect) => {
                let image = match runtime_data.monitors.into_iter().find_map(|mon| {
                    if mon.rect.contains(&rect) {
                        Some(mon)
                    } else {
                        None
                    }
                }) {
                    Some(mon) => {
                        let rect = rect.to_local(&mon.rect);
                        mon.image.crop_imm(
                            rect.x as u32,
                            rect.y as u32,
                            rect.width as u32,
                            rect.height as u32,
                        )
                    }
                    None => runtime_data.image.crop_imm(
                        (rect.x as f32 * runtime_data.scale_factor) as u32,
                        (rect.y as f32 * runtime_data.scale_factor) as u32,
                        (rect.width as f32 * runtime_data.scale_factor) as u32,
                        (rect.height as f32 * runtime_data.scale_factor) as u32,
                    ),
                };

                return Some(image);
            }
            ExitState::None => (),
        }
    }
}
