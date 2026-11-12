//! # splash-page
//!
//! A lightweight splash screen built on top of
//! [FLTK](https://github.com/fltk-rs/fltk-rs).
//!
//! ## Features
//!
//! - Background image embedded as a **Base64** string — no external assets;
//! - Text lines with left / center / right alignment, custom color,
//!   font size, and bold weight;
//! - Any text line can be bound to a URL: the cursor turns into a hand on
//!   hover and a click opens the link in the system browser;
//! - Driven by a TOML config file, easy to hot-reload or ship with a bundle;
//! - Automatically probes for CJK fonts on Windows / macOS / Linux to avoid
//!   tofu characters.
//!
//! ## Quick start
//!
//! ```no_run
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     splash_page::init("splash.toml")?;
//!     Ok(())
//! }
//! ```
//!
//! ## Config example
//!
//! ```toml
//! enabled = true
//! duration_ms = 3000
//! width = 900
//! height = 600
//! background_image_base64 = "data:image/png;base64,iVBORw0KGgo..."
//!
//! [[lines]]
//! text = "My Application"
//! color = "#FFFFFF"
//! font_size = 48
//! bold = true
//! alignment = "center"
//!
//! [[lines]]
//! text = "https://example.com"
//! color = "#4FC3F7"
//! font_size = 20
//! alignment = "center"
//! link = "https://example.com"
//! ```
//!
//! ## Caveats
//!
//! - [`init`] creates and runs its own `fltk::app::App` event loop, so it
//!   **must not** be called from a process that already runs FLTK. To embed
//!   a splash into a host app, split window construction and the event loop
//!   following the shape of [`init`].
//! - When the timeout fires only the window is hidden; `app::quit()` is
//!   **not** called, so a host event loop is left untouched.
//! - Fonts are loaded via `Font::load_font` and used by name. The global
//!   `Font::Screen` / `Font::ScreenBold` are **not** modified, so the host
//!   application's font settings are never polluted.
//! - The background image is stretched by FLTK at draw time and is **not**
//!   pre-scaled, saving a `width × height × 3` buffer. On low-end devices
//!   where redraw stutters, consider pre-scaling to the window size.

mod base64_image;
mod splash_config;

use base64_image::decode_base64_image;
use fltk::{app, draw, enums::*, frame::Frame, prelude::*, window::Window};
use splash_config::{Alignment, SplashConfig};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// ========== Font and color helpers ==========

/// Load CJK fonts from the system and return their names.
///
/// Returns `(regular, bold)`. A `None` entry means no matching font was
/// found; the caller should fall back to a default font. The global
/// `Font::Screen` / `Font::ScreenBold` are **not** modified, so embedding
/// this crate into a host application never changes the host's fonts.
fn load_chinese_fonts() -> (Option<String>, Option<String>) {
    let regular_candidates = [
        "C:/Windows/Fonts/simkai.ttf",
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/msyh.ttf",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/System/Library/Fonts/PingFang.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    ];
    let regular = regular_candidates
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .and_then(|p| Font::load_font(p).ok());

    let bold_candidates = [
        "C:/Windows/Fonts/simhei.ttf",
        "C:/Windows/Fonts/msyhbd.ttc",
        "/System/Library/Fonts/PingFang.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
    ];
    let bold = bold_candidates
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .and_then(|p| Font::load_font(p).ok());

    (regular, bold)
}

/// Pick a font for one line.
///
/// Resolution order:
/// 1. If `line.font_family` is non-empty and FLTK knows that font by name,
///    use it directly.
/// 2. Otherwise fall back to the auto-detected CJK font for the given
///    `bold` flag.
/// 3. If no CJK font was loaded, fall back to `Helvetica` / `HelveticaBold`.
fn pick_font(family: &str, regular: &Option<String>, bold: &Option<String>, is_bold: bool) -> Font {
    // 1. Explicit family from config.
    if !family.trim().is_empty() {
        let font = Font::by_name(family);
        // `Font::by_name` always returns a Font; verify it round-trips to a
        // known name so we don't silently draw with a bogus font id.
        if font != Font::Helvetica || family.eq_ignore_ascii_case("helvetica") {
            return font;
        }
    }

    // 2. Auto-detected CJK font.
    let name = if is_bold { bold } else { regular };
    if let Some(n) = name {
        return Font::by_name(n);
    }

    // 3. Built-in fallback.
    if is_bold {
        Font::HelveticaBold
    } else {
        Font::Helvetica
    }
}

/// Parse a color of the form `#RRGGBB` or `RRGGBB`.
///
/// Invalid input is logged and falls back to white instead of silently
/// swallowing the config error.
fn parse_hex_color(s: &str) -> Color {
    let trimmed = s.trim().trim_start_matches('#');
    if trimmed.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&trimmed[0..2], 16),
            u8::from_str_radix(&trimmed[2..4], 16),
            u8::from_str_radix(&trimmed[4..6], 16),
        ) {
            return Color::from_rgb(r, g, b);
        }
    }
    eprintln!("[splash-page] invalid color {s:?}, falling back to white");
    Color::White
}

// ========== Clickable line ==========

/// Hit region of a clickable text line.
///
/// Coordinates match the draw coordinates (relative to the frame). `y` is
/// the top edge of the text block, i.e. the baseline minus `font_size`.
#[derive(Clone)]
struct ClickableLine {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    url: String,
}

/// Shared hit test used by both drawing and event handling, so the two
/// never drift apart.
fn hit_test(list: &[ClickableLine], mx: i32, my: i32) -> Option<&ClickableLine> {
    list.iter()
        .find(|c| mx >= c.x && mx <= c.x + c.w && my >= c.y && my <= c.y + c.h)
}

// ========== Entry point ==========

/// Read a TOML config and show the splash window until the timeout fires
/// or the window is closed.
///
/// # Arguments
///
/// - `toml_path`: path to the config file.
///
/// # Behavior
///
/// - If `enabled = false`, returns `Ok(())` immediately without creating a
///   window;
/// - If the background image cannot be decoded, returns an error (and logs
///   it to stderr);
/// - The window is centered, borderless, and modal;
/// - After `duration_ms`, only the window is hidden; `app::quit()` is
///   **not** called;
/// - Creates and runs its own `fltk::app::App`, so it **must not** be
///   called from an existing FLTK event loop.
///
/// # Examples
///
/// ```no_run
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     splash_page::init("splash.toml")?;
///     Ok(())
/// }
/// ```
///
/// # Errors
///
/// - The config file cannot be read or parsed as TOML;
/// - The background image cannot be Base64-decoded, its format is unknown,
///   or its pixel layout is unsupported;
/// - FLTK fails to initialize or the event loop returns an error.
pub fn init(toml_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config = SplashConfig::from_toml_file(toml_path)?;

    if !config.enabled {
        return Ok(());
    }

    let win_w = config.width as i32;
    let win_h = config.height as i32;
    let duration_ms = config.duration_ms;

    let app = app::App::default();

    // Only load font names; do not touch the global Font::Screen / ScreenBold.
    let (regular_font, bold_font) = load_chinese_fonts();

    let (sw, sh) = app::screen_size();
    let pos_x = ((sw as i32 - win_w) / 2).max(0);
    let pos_y = ((sh as i32 - win_h) / 2).max(0);

    let mut win = Window::new(pos_x, pos_y, win_w, win_h, "");
    win.set_border(false);
    win.set_color(Color::Black);
    win.make_modal(true);

    // Decode the background image: surface a clear error instead of exiting
    // silently.
    let bg_image = match decode_base64_image(&config.background_image_base64) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("[splash-page] failed to decode background image: {e}");
            return Err(e);
        }
    };

    // The image is deliberately not pre-scaled to the window size.
    // FLTK's SharedImage::draw(x, y, w, h) already stretches at draw time;
    // pre-scaling would only waste an extra win_w*win_h*3 buffer.
    // Only consider pre-scaling if stutters are observed in practice.
    let bg_image = Rc::new(RefCell::new(bg_image));

    let mut frame = Frame::new(0, 0, win_w, win_h, "");
    frame.set_frame(FrameType::NoBox);

    // The config is read-only, no RefCell needed.
    let config_rc = Rc::new(config);
    let clickables: Rc<RefCell<Vec<ClickableLine>>> = Rc::new(RefCell::new(Vec::new()));
    let last_hovering: Rc<Cell<bool>> = Rc::new(Cell::new(false));

    // Drawing
    {
        let config_ref = config_rc.clone();
        let bg_image = bg_image.clone();
        let clickables = clickables.clone();
        let regular_font = regular_font.clone();
        let bold_font = bold_font.clone();

        frame.draw(move |f| {
            clickables.borrow_mut().clear();

            // Background: stretched at draw time to save memory.
            {
                let mut img = bg_image.borrow_mut();
                img.draw(f.x(), f.y(), f.w(), f.h());
            }

            // Text
            let mut y = 20i32;

            for line in &config_ref.lines {
                if line.text.is_empty() {
                    y += (line.font_size as i32) + 10;
                    continue;
                }

                let font = pick_font(
                    &line.font_family, // 新增：配置里的字体名
                    &regular_font,
                    &bold_font,
                    line.bold,
                );
                draw::set_font(font, line.font_size as i32);
                draw::set_draw_color(parse_hex_color(&line.color));

                let (text_w, _) = draw::measure(&line.text, false);

                let x = match line.alignment {
                    Alignment::Left => 20,
                    Alignment::Center => (f.w() - text_w) / 2,
                    Alignment::Right => f.w() - text_w - 20,
                };

                // draw_text's y is the baseline.
                let baseline_y = y + line.font_size as i32;
                draw::draw_text(&line.text, x, baseline_y);

                if let Some(url) = &line.link {
                    // Hit region = font_size above the baseline, matching
                    // the visible text block and the hit_test semantics.
                    clickables.borrow_mut().push(ClickableLine {
                        x,
                        y: baseline_y - line.font_size as i32,
                        w: text_w,
                        h: line.font_size as i32,
                        url: url.clone(),
                    });
                }

                y += (line.font_size as i32) + 10;
            }
        });
    }

    // Mouse events: click and hover.
    // The frame sits at (0, 0), so app::event_x/y relative to the frame
    // matches the draw coordinates. If the frame ever moves, change this to
    // app::event_x() - f.x().
    {
        let clickables = clickables.clone();
        let last_hovering = last_hovering.clone();

        frame.handle(move |f, ev| match ev {
            Event::Push => {
                let (mx, my) = (app::event_x(), app::event_y());
                if let Some(c) = hit_test(&clickables.borrow(), mx, my) {
                    let _ = open::that(&c.url);
                    return true;
                }
                false
            }
            Event::Move => {
                let (mx, my) = (app::event_x(), app::event_y());
                let hovering = hit_test(&clickables.borrow(), mx, my).is_some();

                // Only update the cache when the state actually changed AND
                // we managed to grab the window. This avoids the cursor
                // getting stuck when f.window() transiently returns None.
                if hovering != last_hovering.get() {
                    if let Some(mut w) = f.window() {
                        w.set_cursor(if hovering {
                            Cursor::Hand
                        } else {
                            Cursor::Default
                        });
                        last_hovering.set(hovering);
                    }
                }
                false
            }
            _ => false,
        });
    }

    win.end();
    win.show();

    // Timeout: hide only this window; never call app::quit() so a host
    // event loop is left untouched.
    let mut win_handle = win.clone();
    let secs = duration_ms as f64 / 1000.0;
    app::add_timeout3(secs, move |_| {
        win_handle.hide();
    });

    app.run()?;
    Ok(())
}
