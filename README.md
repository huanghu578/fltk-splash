# fltk-splash

A lightweight splash screen library built on top of [FLTK](https://github.com/fltk-rs/fltk-rs).
- Background image embedded as a **Base64** string — no external assets;
- Text lines with left / center / right alignment, custom color, font size,  font family, and bold weight;
- Any text line can be bound to a URL: the cursor turns into a hand on hover and a click opens the link in the system browser;
- Driven by a TOML config file, easy to hot-reload or ship with a bundle;
- Automatically probes for CJK fonts on Windows / macOS / Linux to avoid   tofu characters.

## Installation

```toml
[dependencies]
fltk-splash = "0.1.0"
```
FLTK is required at runtime; fltk-rs statically builds FLTK by default, so no manual SDK install is needed.


## Quick start
```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    splash_page::init("splash.toml")?;
    Ok(())
}
```
## Example splash.toml:

```toml
enabled = true
duration_ms = 3000
width = 900
height = 600
background_image_base64 = "data:image/png;base64,iVBORw0KGgo..."

[[lines]]
text = "My Application"
color = "#FFFFFF"
font_family = "Consolas"
font_size = 48
bold = true
alignment = "center"

[[lines]]
text = "https://example.com"
color = "#4FC3F7"
font_size = 20
alignment = "center"
link = "https://example.com"
```
### Configuration
#### Top level
| Field | Type | Required | Description |
|---|---|---|---|
| `enabled` | bool | yes | When `false`, `init` returns without creating a window |
| `duration_ms` | u64 | yes | Display duration in milliseconds |
| `width` | u32 | yes | Window width in pixels |
| `height` | u32 | yes | Window height in pixels |
| `background_image_base64` | string | yes | Base64 background image, optional `data:` prefix |
| `lines` | array | no | Text lines |
#### [[lines]]
| Field | Type | Default | Description |
|---|---|---|---|
| `text` | string | `""` | Text; empty occupies a line but is not drawn |
| `color` | string | `#DDDDDD` | `#RRGGBB` or `RRGGBB` |
| `font_family` | string | `""` | Font family name via `Font::by_name`; empty → auto-detected CJK font by `bold` |
| `font_size` | u32 | `38` | Font size in pixels |
| `alignment` | `"left"` / `"center"` / `"right"` | `"left"` | Horizontal alignment |
| `bold` | bool | `false` | Bold weight |
| `link` | string | none | URL opened on click; absent → not clickable |

> `font_family` should be a font installed on the target system; unknown names fall back to FLTK's default font.

### Supported image formats
Detected by magic number, independent of file extension or MIME:
-PNG
-JPEG
-GIF (first frame only)
-BMP

The background image is stretched to the window size by FLTK at draw time
and is not pre-scaled, saving a width × height × 3 buffer. If redraws
stutter on low-end devices, pre-scale the image yourself.

### Behavior notes
The window is centered, borderless, and modal, with a black background;

After duration_ms, only the window is hidden; app::quit() is not called, so an embedding host's event loop is left untouched;

init creates and runs its own fltk::app::App, so it must not be called from a process that already runs FLTK;

CJK fonts are loaded via Font::load_font and used by name. The global Font::Screen / Font::ScreenBold are not modified, so the host application's font settings are never polluted;

Invalid colors (not #RRGGBB) are logged and fall back to white instead of being silently swallowed;

Per-line font resolution: a non-empty font_family is used directly via Font::by_name; otherwise the auto-detected CJK font is used (regular or bold according to bold); if no CJK font was found, Helvetica / HelveticaBold is the final fallback.

Platform font probing
| Platform | Regular | Bold |
|---|---|---|
| Windows | simkai / msyh / msyh | simhei / msyhbd |
| macOS | STHeiti Light / PingFang | PingFang |
| Linux | wqy-zenhei / NotoSansCJK-Regular | NotoSansCJK-Bold / wqy-zenhei |
Falls back to Helvetica / HelveticaBold when none are found.

### Example
sh
cargo run --example basic -- path/to/splash.toml
## License
MIT OR Apache-2.0