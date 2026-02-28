use std::io::{self, Cursor, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use image::codecs::gif::GifDecoder;
use image::imageops::FilterType;
use image::{AnimationDecoder, DynamicImage, GenericImageView, ImageFormat, Pixel, RgbaImage};

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

fn is_gif_source(source: &str) -> bool {
    if source.starts_with("http://") || source.starts_with("https://") {
        let path = source.split('?').next().unwrap_or(source);
        path.to_lowercase().ends_with(".gif")
    } else {
        ImageFormat::from_path(source)
            .map(|f| f == ImageFormat::Gif)
            .unwrap_or(false)
    }
}

fn load_image(source: &str) -> DynamicImage {
    let bytes = load_bytes(source);
    image::load_from_memory(&bytes).unwrap_or_else(|e| {
        eprintln!("Failed to decode image: {e}");
        std::process::exit(1);
    })
}

fn load_bytes(source: &str) -> Vec<u8> {
    if source.starts_with("http://") || source.starts_with("https://") {
        let response = ureq::get(source).call().unwrap_or_else(|e| {
            eprintln!("Failed to fetch URL '{source}': {e}");
            std::process::exit(1);
        });
        let mut bytes = Vec::new();
        response
            .into_body()
            .into_reader()
            .take(50 * 1024 * 1024)
            .read_to_end(&mut bytes)
            .unwrap_or_else(|e| {
                eprintln!("Failed to read response: {e}");
                std::process::exit(1);
            });
        bytes
    } else {
        std::fs::read(source).unwrap_or_else(|e| {
            eprintln!("Failed to read file '{source}': {e}");
            std::process::exit(1);
        })
    }
}

fn load_gif_frames(source: &str) -> Vec<(RgbaImage, Duration)> {
    let bytes = load_bytes(source);
    let decoder = GifDecoder::new(Cursor::new(bytes)).unwrap_or_else(|e| {
        eprintln!("Failed to decode GIF: {e}");
        std::process::exit(1);
    });

    decoder
        .into_frames()
        .map(|result| {
            let frame = result.unwrap_or_else(|e| {
                eprintln!("Failed to decode GIF frame: {e}");
                std::process::exit(1);
            });
            let (numer, denom) = frame.delay().numer_denom_ms();
            let delay = if denom == 0 {
                Duration::from_millis(100)
            } else {
                Duration::from_millis((numer as u64) / (denom as u64))
            };
            // GIFs with 0 delay often mean "as fast as possible", default to 100ms
            let delay = if delay.is_zero() {
                Duration::from_millis(100)
            } else {
                delay
            };
            (frame.into_buffer(), delay)
        })
        .collect()
}

struct TermSize {
    width: u32,
    height: u32,
}

fn get_term_size(width_override: Option<u32>) -> TermSize {
    let (tw, th) = terminal_size::terminal_size()
        .map(|(w, h)| (w.0 as u32, h.0 as u32))
        .unwrap_or((80, 24));
    TermSize {
        width: width_override.unwrap_or_else(|| tw.saturating_sub(1)),
        height: th,
    }
}

fn compute_render_dimensions(orig_w: u32, orig_h: u32, term: &TermSize) -> (u32, u32) {
    let max_pixel_height = term.height.saturating_sub(1) * 2;

    let mut new_width = term.width.max(1);
    let mut new_height = (orig_h as f64 / orig_w as f64 * new_width as f64).round() as u32;

    if new_height > max_pixel_height {
        new_height = max_pixel_height;
        new_width = (orig_w as f64 / orig_h as f64 * new_height as f64).round() as u32;
    }

    // Ensure even height for clean row pairing
    let new_height = if new_height % 2 == 1 { new_height + 1 } else { new_height };
    (new_width, new_height)
}

fn render_frame<W: Write>(img: &DynamicImage, truecolor: bool, out: &mut W) {
    let (w, h) = img.dimensions();
    for y in (0..h).step_by(2) {
        for x in 0..w {
            let top = img.get_pixel(x, y).to_rgb();
            let bottom = if y + 1 < h {
                img.get_pixel(x, y + 1).to_rgb()
            } else {
                top
            };

            if truecolor {
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
}

fn main() {
    let args = Args::parse();
    let term = get_term_size(args.width);

    if is_gif_source(&args.source) {
        let raw_frames = load_gif_frames(&args.source);

        if raw_frames.len() <= 1 {
            // Single-frame GIF: render as static image
            let img = if let Some((buf, _)) = raw_frames.into_iter().next() {
                DynamicImage::ImageRgba8(buf)
            } else {
                eprintln!("GIF has no frames");
                std::process::exit(1);
            };
            let (orig_w, orig_h) = img.dimensions();
            let (nw, nh) = compute_render_dimensions(orig_w, orig_h, &term);
            let img = img.resize_exact(nw, nh, FilterType::Lanczos3);
            let stdout = io::stdout();
            let mut out = io::BufWriter::new(stdout.lock());
            render_frame(&img, args.truecolor, &mut out);
            out.flush().unwrap();
            return;
        }

        // Pre-resize all frames
        let first = &raw_frames[0].0;
        let (orig_w, orig_h) = (first.width(), first.height());
        let (nw, nh) = compute_render_dimensions(orig_w, orig_h, &term);
        let row_count = nh / 2;

        let frames: Vec<(DynamicImage, Duration)> = raw_frames
            .into_iter()
            .map(|(buf, delay)| {
                let img = DynamicImage::ImageRgba8(buf).resize_exact(nw, nh, FilterType::Lanczos3);
                (img, delay)
            })
            .collect();

        // Set up Ctrl+C handler
        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();
        ctrlc::set_handler(move || {
            r.store(false, Ordering::SeqCst);
        })
        .expect("Failed to set Ctrl+C handler");

        let stdout = io::stdout();
        let mut out = io::BufWriter::new(stdout.lock());

        // Hide cursor during animation
        write!(out, "\x1b[?25l").unwrap();

        let mut first_frame = true;
        while running.load(Ordering::SeqCst) {
            for (img, delay) in &frames {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                // Move cursor up to overwrite previous frame (except for first frame)
                if !first_frame {
                    write!(out, "\x1b[{}A", row_count).unwrap();
                }
                first_frame = false;

                render_frame(img, args.truecolor, &mut out);
                out.flush().unwrap();
                std::thread::sleep(*delay);
            }
        }

        // Clean up: show cursor, reset colors
        write!(out, "\x1b[?25h\x1b[0m").unwrap();
        out.flush().unwrap();
    } else {
        // Static image path (unchanged)
        let img = load_image(&args.source);
        let (orig_w, orig_h) = img.dimensions();
        let (nw, nh) = compute_render_dimensions(orig_w, orig_h, &term);
        let img = img.resize_exact(nw, nh, FilterType::Lanczos3);

        let stdout = io::stdout();
        let mut out = io::BufWriter::new(stdout.lock());
        render_frame(&img, args.truecolor, &mut out);
        out.flush().unwrap();
    }
}
