use std::io::{self, Read, Write};
use std::path::PathBuf;

use clap::Parser;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, Pixel};

#[derive(Parser)]
#[command(name = "termimg", about = "Render images in the terminal using colored Unicode blocks")]
struct Args {
    /// Path to image file or URL (http/https)
    source: String,

    /// Output width in terminal columns (defaults to terminal width)
    #[arg(short, long)]
    width: Option<u32>,

    /// Use 24-bit truecolor (for iTerm2, Kitty, etc). Default is 256-color for Terminal.app compatibility.
    #[arg(long)]
    truecolor: bool,
}

/// Convert an 8-bit channel value (0-255) to the 6-level 256-color cube (0-5)
fn to_ansi_level(v: u8) -> u8 {
    ((v as u16 * 5 + 127) / 255) as u8
}

/// Convert RGB to a 256-color palette index (16-231 color cube)
fn rgb_to_256(r: u8, g: u8, b: u8) -> u8 {
    16 + 36 * to_ansi_level(r) + 6 * to_ansi_level(g) + to_ansi_level(b)
}

fn load_image(source: &str) -> DynamicImage {
    if source.starts_with("http://") || source.starts_with("https://") {
        let response = ureq::get(source).call().unwrap_or_else(|e| {
            eprintln!("Failed to fetch URL '{source}': {e}");
            std::process::exit(1);
        });
        let mut bytes = Vec::new();
        response
            .into_body()
            .into_reader()
            .take(50 * 1024 * 1024) // 50MB limit
            .read_to_end(&mut bytes)
            .unwrap_or_else(|e| {
                eprintln!("Failed to read response: {e}");
                std::process::exit(1);
            });
        image::load_from_memory(&bytes).unwrap_or_else(|e| {
            eprintln!("Failed to decode image from URL: {e}");
            std::process::exit(1);
        })
    } else {
        let path = PathBuf::from(source);
        image::open(&path).unwrap_or_else(|e| {
            eprintln!("Failed to open image '{}': {e}", path.display());
            std::process::exit(1);
        })
    }
}

fn main() {
    let args = Args::parse();

    let img = load_image(&args.source);

    let term_width = args.width.unwrap_or_else(|| {
        terminal_size::terminal_size()
            .map(|(w, _)| (w.0 as u32).saturating_sub(1))
            .unwrap_or(80)
    });

    let term_height = terminal_size::terminal_size()
        .map(|(_, h)| h.0 as u32)
        .unwrap_or(24);
    // Each row of half-blocks = 2 pixels, leave 1 row for the prompt
    let max_rows = term_height.saturating_sub(1);
    let max_pixel_height = max_rows * 2;

    let (orig_w, orig_h) = img.dimensions();
    let mut new_width = term_width.max(1);
    let mut new_height = (orig_h as f64 / orig_w as f64 * new_width as f64).round() as u32;

    // If image is too tall, scale down to fit terminal height
    if new_height > max_pixel_height {
        new_height = max_pixel_height;
        new_width = (orig_w as f64 / orig_h as f64 * new_height as f64).round() as u32;
    }

    // Ensure even height for clean row pairing
    let new_height = if new_height % 2 == 1 { new_height + 1 } else { new_height };

    let img = img.resize_exact(new_width, new_height, FilterType::Lanczos3);

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    for y in (0..new_height).step_by(2) {
        for x in 0..new_width {
            let top = img.get_pixel(x, y).to_rgb();
            let bottom = if y + 1 < new_height {
                img.get_pixel(x, y + 1).to_rgb()
            } else {
                top
            };

            // Background = top pixel, foreground = bottom pixel, char = ▄
            if args.truecolor {
                write!(
                    out,
                    "\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m▄",
                    top[0], top[1], top[2], bottom[0], bottom[1], bottom[2],
                )
            } else {
                write!(
                    out,
                    "\x1b[48;5;{}m\x1b[38;5;{}m▄",
                    rgb_to_256(top[0], top[1], top[2]),
                    rgb_to_256(bottom[0], bottom[1], bottom[2]),
                )
            }
            .unwrap();
        }
        write!(out, "\x1b[0m\n").unwrap();
    }

    out.flush().unwrap();
}
