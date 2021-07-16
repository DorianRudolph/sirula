# Sirula

1. [About](#intro)
2. [Examples](#examples)
3. [Building](#build)
4. [Configuration](#config)

<a name="intro"></a>
## About

Sirula (simple rust launcher) is an app launcher for wayland.  
Currently, the only feature is launching apps from `.desktop` files.  
Feel free to submit pull requests for any feature you like.

I wrote sirula partially to learn rust, so do not expect perfect rust code.  
I'd be happy to hear any criticism of my code.

<a name="examples"></a>
## Examples

You will find a list of all sample configs [here](https://github.com/DorianRudolph/sirula/tree/master/sample-config). 
Sample a is the default config shipped with sirula.

<a name="build"></a>
## Building

- Dependency: [gtk-layer-shell](https://github.com/wmww/gtk-layer-shell)
- Build: `cargo build --release`
  - Optionally, `strip` the binary to reduce size
- Alternatively, install with `cargo install --path .`

<a name="config"></a>
## Configuration

To configure sirula first create the config directory `mkdir ~/.config/sirula`.
From here you can create two files to customise sirula.  
`touch config.toml`  
`touch style.css`

From here you can edit and change these files as you wish or use a sample config as found [here](https://github.com/DorianRudolph/sirula/tree/master/sample-config).

