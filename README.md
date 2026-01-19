# walk_bg
A basic application to display a
[random walk](https://en.wikipedia.org/wiki/Random_walk) as a background image.

walk_bg will work with any wayland compositor that implements
`wlr-layer-shell-unstable-v1`. See [Supported Compositors](#supported-compositors)
for more information.

# Installation
## From releases:
Precompiled binaries are available on the releases page in the right sidebar.

## From source
You need to have Rust and Cargo installed. Then you can clone the repository and build it with Cargo:
```bash
git clone https://github.com/BjarneSeger/walk_bg.git
cargo build --release
```
The binary will be located in `target/release/walk_bg`.
Place it in a folder that is in your path, like ~/.local/bin/ and add an autostart
for your compositor to run `walk_bg`.

# Supported compositors
walk_bg uses `zwlr_layer_shell_v1` which should be supported by most window managers
using wlroots (sway, weston) as well as Hyprland, Cosmic Desktop, KDE Plasma and
probably some others. To find out if you compositor is supported, use `wayland-info`
from the `wayland-utils`-package and check if `zwlr_layer_shell_v1` is listed:
```bash
wayland-info | grep zwlr_layer_shell_v1
```
This should return something like:
```
interface: 'zwlr_layer_shell_v1',                        version:  5, name: 35
```
If it does not, like on GNOME, walk_bg will not work.
