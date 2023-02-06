# WaterShot
A simple wayland native screenshot tool inspired by [Flameshot](https://flameshot.org/).

# Installation
Simply clone the repository and install the program locally with cargo. You will need to have [grim](https://sr.ht/~emersion/grim/),
if it is in a non-standard location you can use `--grim` or `-g` argument to set a custom path. A compositor that implements layer-shell is also a requirement.
```
git clone https://github.com/Kirottu/watershot
cd watershot
cargo install --path .
```

# Usage
Just run the executable. 
Do note that without any arguments, the screenshots are not saved/copied anywhere.

```
Commands:
  path       The path to save the image to
  directory  The directory to save the image to with a generated name
  help       Print this message or the help of the given subcommand(s)

Options:
  -c, --copy     Copy the screenshot after exit
  -g, --grim <GRIM>  Path to the `grim` executable
  -h, --help     Print help
  -V, --version  Print version
```

# Configuration
WaterShot supports configuration of colors, fonts, sizes, etc. via it's config file. The config file is
saved in `~/.config/watershot.ron` and uses the ron config format.

Here is an example config for it:
```
Config(
    handle_radius: 10,
    line_width: 2,
    display_highlight_width: 5,
    selection_color: Color(
        r: 0.38,
        g: 0.68,
        b: 0.94,
        a: 1.0,
    ),
    shade_color: Color(
        r: 0.11,
        g: 0.0,
        b: 0.11,
        a: 0.6,
    ),
    text_color: Color(
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    ),
    size_text_size: 15,
    mode_text_size: 30,
    font_family: "monospace",
)
```
