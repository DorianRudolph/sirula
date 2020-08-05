# Sirula

Sirula (simple rust launcher) is an app launcher for wayland.
Currently, the only feature is launching apps from `.desktop` files.
Feel free to submit pull requests for any feature you like.

I wrote sirula partially to learn rust, so do not expect perfect rust code.
I'd be happy to hear any criticism of my code.

## Building

- Dependency: [gtk-layer-shell](https://github.com/wmww/gtk-layer-shell)
- Build: `cargo build --release`
  - Optionally, `strip` the binary to reduce size
- Alternatively, install with `cargo install --path .`

## Configuration

Use `config.toml` and `style.css` in your `.config/sirula` directory.
See `sample-config` for documentation.