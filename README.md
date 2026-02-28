# terminal-images

Render images in your terminal using colored Unicode block characters.

Uses the half-block character `▄` with ANSI color codes to display images directly in the terminal. Each character cell encodes two vertical pixels — the background color for the top pixel and the foreground color for the bottom — giving surprisingly detailed results.

<img width="1334" height="1229" alt="image" src="https://github.com/user-attachments/assets/61e23453-9040-48d7-a771-d115c9851e4a" />


## Install

```
cargo install --path .
```

## Usage

```
termimg <source> [options]
```

**From a local file:**

```
termimg photo.jpg
termimg ~/Pictures/screenshot.png
```

**From a URL:**

```
termimg https://example.com/image.png
```

**Animated GIFs:**

```
termimg animation.gif
```

GIFs with multiple frames will animate in the terminal, looping until you press Ctrl+C. Single-frame GIFs are rendered as static images.

### Options

| Flag | Description |
|------|-------------|
| `-w, --width <COLS>` | Set output width in columns (defaults to terminal width) |
| `--truecolor` | Use 24-bit color for iTerm2, Kitty, etc. |

### Examples

```
# Render at 60 columns wide
termimg photo.jpg -w 60

# Use truecolor in iTerm2 for maximum color fidelity
termimg photo.jpg --truecolor
```

## Color modes

- **256-color** (default) — Works in macOS Terminal.app and virtually all terminals. Uses the 6x6x6 ANSI color cube.
- **Truecolor** (`--truecolor`) — Full 24-bit RGB. Use this in iTerm2, Kitty, Alacritty, WezTerm, or any terminal that supports it.

## Supported formats

PNG, JPEG, GIF, BMP, TIFF, WebP, and more (via the [image](https://crates.io/crates/image) crate).

## How it works

1. Load the image and detect terminal dimensions
2. Resize to fit within the terminal (width and height), preserving aspect ratio
3. Iterate pixel rows in pairs — for each pair, set the background color to the top pixel and the foreground color to the bottom pixel, then print `▄`
4. Uses Lanczos3 resampling for quality downscaling

For animated GIFs, all frames are pre-resized on load, then played back by overwriting the previous frame using cursor movement escape sequences. The cursor is hidden during playback for a cleaner look.
