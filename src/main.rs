use clap::Parser;
use dialoguer::Select;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

async fn ensure_yt_dlp() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let bin_dir = PathBuf::from("ytpd");
    fs::create_dir_all(&bin_dir)?;

    let yt_dlp_path = if cfg!(windows) {
        bin_dir.join("yt-dlp.exe")
    } else {
        bin_dir.join("yt-dlp")
    };

    if !yt_dlp_path.exists() {
        let url = if cfg!(windows) {
            "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
        } else {
            "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp"
        };

        let response = reqwest::get(url).await?;
        let bytes = response.bytes().await?;
        fs::write(&yt_dlp_path, bytes)?;

        if !cfg!(windows) {
            let mut perms = fs::metadata(&yt_dlp_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&yt_dlp_path, perms)?;
        }
    }

    Ok(yt_dlp_path)
}

const DOWNLOAD_DIR: &str = "Audio";

#[derive(Parser)]
#[command(name = "ytpd")]
#[command(about = "Youtube Music Downloader")]
struct Cli {
    url: Option<String>,
}

#[derive(Clone, Copy)]
enum AudioFormat {
    MP3,
    WAV,
    M4A,
    AAC,
    FLAC,
}

impl AudioFormat {
    fn as_str(&self) -> &'static str {
        match self {
            AudioFormat::MP3 => "mp3",
            AudioFormat::WAV => "wav",
            AudioFormat::M4A => "m4a",
            AudioFormat::AAC => "aac",
            AudioFormat::FLAC => "flac",
        }
    }
}

#[tokio::main]
async fn main() {
    let yt_dlp_path = match ensure_yt_dlp().await {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Failed to setup yt-dlp: {}", e);
            return;
        }
    };

    fs::create_dir_all(DOWNLOAD_DIR).expect("Failed to create Audio directory");

    let cli = Cli::parse();

    let url = if let Some(url) = cli.url {
        url
    } else {
        print!("Enter Youtube URL: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read input");
        input.trim().to_string()
    };

    let is_playlist = url.contains("playlist?list=");
    let options = vec!["Single Song", "Playlist"];
    let selection = Select::new()
        .with_prompt("Choose download type")
        .items(&options)
        .default(0)
        .interact()
        .unwrap();

    if selection == 0 && is_playlist {
        println!("Error: This is a playlist/album URL. Please provide a single song URL for single song download.");
        return;
    }

    let format_options = vec!["MP3", "WAV", "M4A", "AAC", "FLAC"];
    let format_selection = Select::new()
        .with_prompt("Choose audio format")
        .items(&format_options)
        .default(0)
        .interact()
        .unwrap();

    let audio_format = match format_selection {
        0 => AudioFormat::MP3,
        1 => AudioFormat::WAV,
        2 => AudioFormat::M4A,
        3 => AudioFormat::AAC,
        4 => AudioFormat::FLAC,
        _ => AudioFormat::MP3,
    };

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} Downloading... {wide_msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(Duration::from_millis(100));

    let mut args = vec![
        "-x",
        "--audio-format",
        audio_format.as_str(),
        "--audio-quality",
        "0",
        "-P",
        DOWNLOAD_DIR,
        "--no-check-certificates",
        "--no-warnings",
        "--ignore-errors",
    ];

    if selection == 0 {
        args.push("--no-playlist");
    }

    args.push(&url);

    let output = Command::new(&yt_dlp_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute yt-dlp");

    spinner.finish_and_clear();

    if output.status.success() {
        println!("Download completed!");
    } else {
        println!(
            "Download failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
