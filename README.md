# ytpd

A simple and efficient YouTube Music Downloader.
basically yt-dlp with better 'interface' built over it but less options cuz i don't really care about them.

## Features

- Download of entire playlists
- Multiple audio formats (MP3, WAV, M4A, AAC, FLAC)
- Automatic thumbnail embedding
- High-quality audio output
- Download indicator (spinner)

## Installation

```bash
git clone https://github.com/heyimnel/ytpd
cd ytpd
cargo install --path .
```

## Usage

### Interactive Mode
```bash
ytpd
```

### Command Line Mode
```bash
ytpd <URL>
```

### Examples:
```bash
ytpd https://youtube.com/watch?v=...
ytpd https://youtube.com/playlist?list=...
```

## Options

- Choose download location (Current directory or 'Audio' folder)
- Select audio format (MP3, WAV, M4A, AAC, FLAC)
- Enable/disable thumbnail embedding
- Single song or playlist download

## Requirements

1. If you're on macOS then some dependencies are handled by the program (FFmpeg, yt-dlp)
   - if not, [ffmpeg](https://ffmpeg.org/), [yt-dlp](https://github.com/yt-dlp/yt-dlp?tab=readme-ov-file#installation)
2. Rust is required and needs to be installed by the user
   - on macOS u can do `brew install rust` or `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
