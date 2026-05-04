# Sirula

Sirula (simple rust launcher) is an app launcher for wayland.
Currently, the only feature is launching apps from `.desktop` files.
Feel free to submit pull requests for any feature you like.

I wrote sirula partially to learn rust, so do not expect perfect rust code.
I'd be happy to hear any criticism of my code.

## Examples

`sample-config/a`:

[![](sample-config/a/sirula.gif)](https://raw.githubusercontent.com/DorianRudolph/sirula/master/sample-config/sirula.gif)

`sample-config/e`: Overlay in the center of the screen with some simple CSS animations.

[![](sample-config/e/sirula.gif)](https://raw.githubusercontent.com/DorianRudolph/sirula/master/sample-config/sirula.gif)

## Building

- Dependency: [gtk-layer-shell](https://github.com/wmww/gtk-layer-shell)
- Build: `cargo build --release`
  - Optionally, `strip` the binary to reduce size
- Alternatively, install with `cargo install --path .`
- There is also an unofficial [AUR package](https://aur.archlinux.org/packages/sirula-git/)

## Configuration

Use `config.toml` and `style.css` in your `.config/sirula` directory.
See `sample-config` for documentation.

## Launching

Simply run `sirula`.

## Daemon mode

**! Latest git only !**

Run `sirula` with `--daemon` to start the daemon.
To show the window, simply run `sirula` again or use `--toggle` to toggle the window.

`sirula --reload`, `sirula --reload-or-restart`, `Ctrl+R` and `F5` will reload all desktop, config and styling files.

See `--help` for more options.

### DE Integration example: sway

Add this to your sway config file:
```
exec_always sirula --reload-or-restart
bindsym --no-repeat --release Super_L exec sirula --toggle
```
