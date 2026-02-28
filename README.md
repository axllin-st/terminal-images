# terminal-images

Render images, animated GIFs, and live webcam feeds in your terminal using colored Unicode block characters.

Uses the half-block character `▄` with ANSI color codes to display visuals directly in the terminal. Each character cell encodes two vertical pixels — the background color for the top pixel and the foreground color for the bottom — giving surprisingly detailed results.

<img width="1334" height="1229" alt="image" src="https://github.com/user-attachments/assets/61e23453-9040-48d7-a771-d115c9851e4a" />


## Install

```
cargo install --path .
```

## Usage

### Static images

Render any common image format (PNG, JPEG, BMP, TIFF, WebP, and more) from a local file or URL:

```
termimg photo.jpg
termimg ~/Pictures/screenshot.png
termimg https://example.com/image.png
```

### Animated GIFs

Pass a GIF file and it will animate in the terminal, looping until you press Ctrl+C:

```
termimg animation.gif
termimg https://example.com/funny.gif
```

Single-frame GIFs are rendered as static images.

### Webcam

Stream live video from your system camera:

```
termimg --webcam
```

Press Ctrl+C to stop. On macOS, your terminal emulator needs camera permission (System Settings > Privacy & Security > Camera).

### Options

| Flag | Description |
|------|-------------|
| `-w, --width <COLS>` | Set output width in columns (defaults to terminal width) |
| `--truecolor` | Use 24-bit color for iTerm2, Kitty, etc. |
| `--webcam` | Stream from webcam instead of a file |

### Examples

```
# Render at 60 columns wide
termimg photo.jpg -w 60

# Use truecolor in iTerm2 for maximum color fidelity
termimg photo.jpg --truecolor

# Webcam with custom width
termimg --webcam -w 80
```

## Color modes

- **256-color** (default) — Works in macOS Terminal.app and virtually all terminals. Uses the 6x6x6 ANSI color cube.
- **Truecolor** (`--truecolor`) — Full 24-bit RGB. Use this in iTerm2, Kitty, Alacritty, WezTerm, or any terminal that supports it.

## How it works

1. Load the image and detect terminal dimensions
2. Resize to fit within the terminal (width and height), preserving aspect ratio
3. Iterate pixel rows in pairs — for each pair, set the background color to the top pixel and the foreground color to the bottom pixel, then print `▄`
4. Uses Lanczos3 resampling for quality downscaling (Triangle filter for webcam to prioritize speed)

For animated GIFs, all frames are pre-resized on load, then played back by overwriting the previous frame using cursor movement escape sequences. The webcam uses the same overwrite technique, capturing and rendering frames in a continuous loop. The cursor is hidden during playback for a cleaner look.
