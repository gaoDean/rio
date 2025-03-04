# Rio term

> tl;dr: Rio is a terminal built to run everywhere, as a native desktop applications by Rust/WebGPU or even in the browser powered by WebAssembly/WebGPU.

![Rio banner](docs/assets/banner.png)

## Development Status

#### Status

| Status | Meaning |
| :-: | --- |
| 🟢 | Stable and usable. |
| 🔵 | Unstable and usable (minor bugs and incomplete features). |
| 🟠 | Unstable and usable (major bugs and incomplete features). |
| 🟡 | It does builds but provides a very limited usage or is not able to use at all. |
| 🔴 | Development has not started. |

#### Platform

| Platform | Status | Version introduced |
| --- | :-: | --- |
| MacOs _as desktop application_ | 🔵 | `0.0.3` [(see more)](https://raphamorim.io/rio/install/#macos) |
| Web Browser _(WebAssembly)_ | 🟡 | (Sugarloaf is ready but Rio still need to be ported) |
| Linux _as desktop application_ | 🔵 | `0.0.7` [(see more)](https://raphamorim.io/rio/install/#linux) |
| Windows _as desktop application_ | 🔵 | `0.0.8` [(see more)](https://raphamorim.io/rio/install/#windows) |
| Nintendo Switch * | 🔴 | |

_* Nintendo Switch development is just for fun, the goal is to have  the renderer working and the basic features of a terminal._

## Demo Gallery

| ![Demo tmux](docs/assets/demos/demo-tmux.png) | <img src="docs/assets/demos/demo-emacs.png" alt="Demo emacs" width="500px"/> |
| ----------- | ----------- |
| ![Demo linux x11](docs/assets/demos/demo-x11.png) | ![Demo linux wayland](docs/assets/demos/demo-wayland.png) |
| ![Demo text styles](docs/assets/demos/demo-text-styles.png) | ![Demo selection](docs/assets/demos/demo-selection.png) |
| ![Demo Windows 10](docs/assets/demos/demo-windows-10.png) | ![Demo Windows 11](docs/assets/demos/demo-windows-11.png) |

Note: Emojis are rendered with Noto Emoji.

## Sugarloaf

Rio is built over a custom renderer called [Sugarloaf](https://crates.io/crates/sugarloaf), which is responsible for font and style rendering. Sugarloaf demo:

| Native | Web |
| ----------- | ----------- |
| ![Demo Sugarloaf native](sugarloaf/resources/demo-text-big.png) | ![Demo Sugarloaf wasm](sugarloaf/resources/demo-wasm.png) |

## Summary

- [About Rio](#about-rio)
- [Status](#development-status)
- [Configuration file](#configuration-file)
- [Acknowledgments](#acknowledgments)

## About Rio

Website: https://raphamorim.io/rio

> If you are using or want to help in any way please consider to donate via [Github Sponsors](https://github.com/sponsors/raphamorim).
> 
> Rio would not be possible without [few acknowledgements](#acknowledgements) and specially [Alacritty](https://github.com/alacritty/alacritty/), since a lot of Rio functionalities (e.g: ANSI parser, events, grid system) was originally written (and still uses a good amount) of Alacritty code.
>

Read about Rio in the official website: https://raphamorim.io/rio/docs.

## Configuration File

Configuration file is documented in Rio official website: https://raphamorim.io/rio/docs#configuration-file.

## Acknowledgments

- Alacritty 🥇
- Rio logo was made using _Adobe Sketchbook_ on iPad.
- The default color palette is based on the colors of [ui.dev](https://ui.dev/).
- Text glyph render is from https://github.com/hecrj/wgpu_glyph
- https://github.com/wez/wezterm
- https://www.gaijin.at/en/infos/ascii-ansi-character-table#asciicontrol
- https://en.wikipedia.org/wiki/ANSI_escape_code
- https://www.scratchapixel.com/lessons/3d-basic-rendering/perspective-and-orthographic-projection-matrix/orthographic-projection-matrix.html
