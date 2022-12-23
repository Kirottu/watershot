use std::f64::consts::PI;

use gdk::prelude::*;
use gtk::prelude::*;

use crate::{handles, traits::*, types::*, RUNTIME_DATA};

/// Drawing of the overlay containing the selection, shade and other content
pub fn draw(selection_overlay: &gtk::DrawingArea, ctx: &gtk::cairo::Context) -> Inhibit {
    RUNTIME_DATA.with(|runtime_data| {
        let runtime_data = runtime_data.borrow();

        // Get the application window, looks horrible but works
        let window = selection_overlay
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .dynamic_cast::<gtk::ApplicationWindow>()
            .unwrap();

        // Get the monitor
        let monitor = &runtime_data.windows[&window].monitor;

        // Set some values used later
        ctx.set_source_rgba(
            runtime_data.config.shade_color.r,
            runtime_data.config.shade_color.g,
            runtime_data.config.shade_color.b,
            runtime_data.config.shade_color.a,
        );
        ctx.select_font_face(
            &runtime_data.config.font_family,
            gdk::cairo::FontSlant::Normal,
            gdk::cairo::FontWeight::Normal,
        );
        // Draw different things based on the selection state
        //
        // Rectangle(Some) => Draw shade and a highlighted section for the selection, with all bells and whistles
        // Display(Some) => Draw shade and highlight selected monitor
        // Display(None) | Rectangle(None) => Draw shade and text describing the current mode
        match &runtime_data.selection {
            Selection::Rectangle(Some(selection)) => {
                if selection
                    .extents
                    .to_rect()
                    .intersects(&monitor.geometry().into())
                {
                    let extents = selection.extents.to_local(&monitor);
                    let rect = extents.to_rect();

                    // Draw dimmed background
                    ctx.set_fill_rule(gdk::cairo::FillRule::EvenOdd);
                    ctx.rectangle(
                        0.0,
                        0.0,
                        monitor.geometry().width() as f64,
                        monitor.geometry().height() as f64,
                    );
                    ctx.rectangle(
                        rect.x as f64,
                        rect.y as f64,
                        rect.width as f64,
                        rect.height as f64,
                    );
                    ctx.fill().unwrap();

                    // Draw selection outline
                    ctx.set_source_rgba(
                        runtime_data.config.selection_color.r,
                        runtime_data.config.selection_color.g,
                        runtime_data.config.selection_color.b,
                        runtime_data.config.selection_color.a,
                    );
                    ctx.move_to(extents.start_x as f64, extents.start_y as f64);
                    ctx.set_line_width(runtime_data.config.line_width as f64);
                    ctx.line_to(extents.end_x as f64, extents.start_y as f64);
                    ctx.line_to(extents.end_x as f64, extents.end_y as f64);
                    ctx.line_to(extents.start_x as f64, extents.end_y as f64);
                    ctx.line_to(extents.start_x as f64, extents.start_y as f64);
                    ctx.stroke().unwrap();

                    // Draw drag handles
                    for (x, y, _) in handles!(extents) {
                        ctx.arc(
                            *x as f64,
                            *y as f64,
                            runtime_data.config.handle_radius as f64,
                            0.0,
                            2.0 * PI,
                        );
                        ctx.fill().unwrap();
                    }

                    // Draw the size text
                    let text = format!("{}x{}", rect.width, rect.height);

                    ctx.set_source_rgba(
                        runtime_data.config.text_color.r,
                        runtime_data.config.text_color.g,
                        runtime_data.config.text_color.b,
                        runtime_data.config.text_color.a,
                    );
                    ctx.set_font_size(runtime_data.config.size_text_size as f64);
                    let text_extents = ctx.text_extents(&text).unwrap();
                    ctx.move_to(
                        rect.x as f64 + rect.width as f64 / 2.0 - text_extents.width() / 2.0,
                        (rect.y as f64
                            - text_extents.height()
                            - runtime_data.config.handle_radius as f64)
                            .abs(),
                    );
                    ctx.show_text(&text).unwrap();
                } else {
                    ctx.paint().unwrap();
                }
            }
            Selection::Display(Some(selection)) => {
                if selection.window == window {
                    let geometry = monitor.geometry();
                    ctx.set_source_rgba(
                        runtime_data.config.selection_color.r,
                        runtime_data.config.selection_color.g,
                        runtime_data.config.selection_color.b,
                        runtime_data.config.selection_color.a,
                    );
                    // Draw monitor outline
                    ctx.set_line_width(runtime_data.config.display_highlight_width as f64 * 2.0);
                    ctx.move_to(0.0, 0.0);
                    ctx.line_to(geometry.width() as f64, 0.0);
                    ctx.line_to(geometry.width() as f64, geometry.height() as f64);
                    ctx.line_to(0.0 as f64, geometry.height() as f64);
                    ctx.line_to(0.0, 0.0);
                    ctx.stroke().unwrap();
                } else {
                    ctx.paint().unwrap()
                }
            }
            _ => {
                // Draw text stating the current mode when no selection is present
                let geometry = monitor.geometry();
                ctx.paint().unwrap();
                ctx.set_source_rgba(
                    runtime_data.config.text_color.r,
                    runtime_data.config.text_color.g,
                    runtime_data.config.text_color.b,
                    runtime_data.config.text_color.a,
                );
                ctx.set_font_size(runtime_data.config.mode_text_size as f64);
                let text = match runtime_data.selection {
                    Selection::Rectangle(_) => "RECTANGLE MODE",
                    Selection::Display(_) => "DISPLAY MODE",
                };
                let text_extents = ctx.text_extents(text).unwrap();
                ctx.move_to(
                    (geometry.width() as f64 - text_extents.width()) / 2.0,
                    geometry.height() as f64 / 2.0,
                );
                ctx.show_text(text).unwrap();
            }
        }
    });
    Inhibit(false)
}
