//! Minimal runnable example.
//!
//! Usage:
//!
//! ```sh
//! cargo run --example basic -- path/to/splash.toml
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "splash.toml".to_string());
    splash_page::init(&path)?;

    Ok(())
}
