use std::{error::Error, io::Cursor, process::Command};

use ashpd::desktop::screenshot::Screenshot;
use async_std::task::block_on;
use image::DynamicImage;

use crate::types::Args;

pub(crate) fn grim(args: &Args) -> Result<DynamicImage, Box<dyn Error>> {
    let output = Command::new(args.grim.as_ref().unwrap_or(&"grim".to_string()))
        .arg("-t")
        .arg("ppm")
        .arg("-")
        .output()
        .map_err(|_| Box::<dyn Error>::from("Failed to run grim command!"))?
        .stdout;

    image::io::Reader::with_format(Cursor::new(output), image::ImageFormat::Pnm)
        .decode()
        .map_err(|_| Box::<dyn Error>::from("Failed to parse grim image!"))
}

pub(crate) fn portal() -> Result<DynamicImage, Box<dyn Error>> {
    let future = Screenshot::request().interactive(false).modal(false).send();
    // TODO: consider adopting Futures elsewhere and turning this into an async function
    let request = block_on(future)
        .map_err(|_| Box::<dyn Error>::from("Failed to send Screenshot request!"))?;

    let screenshot = request
        .response()
        .map_err(|_| Box::<dyn Error>::from("Screenshot request failed!"))?;
    let uri = screenshot.uri();

    if uri.scheme() != "file" {
        // we're double-checking here to be extra cautious, however,
        // upstream docs say it should always be a file: https://flatpak.github.io/xdg-desktop-portal/#gdbus-org.freedesktop.portal.Screenshot
        return Err(Box::<dyn Error>::from(
            "URL in Screenshot response is unsupported!",
        ));
    }
    image::io::Reader::open(uri.path())
        .map_err(|_| Box::<dyn Error>::from("Failed to open Screenshot response!"))?
        .decode()
        .map_err(|_| Box::<dyn Error>::from("Failed to decode Screenshot response!"))
}
