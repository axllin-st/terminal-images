use std::io::{self, Write};
use std::path::PathBuf;

use clap::Parser;
use image::imageops::FilterType;
use image::{GenericImageView, Pixel};

#[derive(Parser)]
#[command(name = "termimg", about = "Render images in the terminal using colored Unicode blocks")]
struct Args {
    /// Path to the image file
    image_path: PathBuf,

    /// Output width in terminal columns (defaults to terminal width)
    #[arg(short, long)]
    width: Option<u32>,
}

fn main() {
    let args = Args::parse();

    let img = image::open(&args.image_path).unwrap_or_else(|e| {
        eprintln!("Failed to open image '{}': {e}", args.image_path.display());
        std::process::exit(1);
    });

    let term_width = args.width.unwrap_or_else(|| {
        terminal_size::terminal_size()
            .map(|(w, _)| w.0 as u32)
            .unwrap_or(80)
    });

    let (orig_w, orig_h) = img.dimensions();
    let new_width = term_width;
    // Each character cell is ~2 pixels tall, so we halve the height ratio
    let new_height = (orig_h as f64 / orig_w as f64 * new_width as f64 * 0.5).round() as u32;
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
            write!(
                out,
                "\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m▄",
                top[0], top[1], top[2], bottom[0], bottom[1], bottom[2],
            )
            .unwrap();
        }
        write!(out, "\x1b[0m\n").unwrap();
    }

    out.flush().unwrap();
}
