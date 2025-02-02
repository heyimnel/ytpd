use crate::setup::check_dependencies;
use clap::Parser;
mod setup;
use dialoguer::Select;
use futures::future::join_all;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task;

fn get_download_directory() -> Result<String, std::io::Error> {
    let options = vec!["Create 'Audio' folder", "Current directory"];
    let selection = Select::new()
        .with_prompt("Choose download location")
        .items(&options)
        .default(0)
        .interact()
        .unwrap();

    let download_dir = match selection {
        0 => {
            fs::create_dir_all("Audio")?;
            "Audio"
        }
        _ => ".",
    };

    Ok(download_dir.to_string())
}

fn should_download_thumbail() -> bool {
    let options = vec!["Yes", "No"];
    let selection = Select::new()
        .with_prompt("Download thumbnail?")
        .items(&options)
        .default(0)
        .interact()
        .unwrap();

    selection == 0
}

async fn get_playlist_urls(
    yt_dlp_path: PathBuf,
    playlist_url: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let output = Command::new(&yt_dlp_path)
        .args([
            "--flat-playlist",
            "--get-id",
            "--no-warnings",
            "--no-check-certificates",
            "--ignore-errors",
            playlist_url,
        ])
        .output()?;

    if !output.status.success() {
        println!(
            "Error getting playlist: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Err("Failed to get playlist URLs".into());
    }

    let urls: Vec<String> = String::from_utf8(output.stdout)?
        .lines()
        .filter(|line| !line.is_empty())
        .map(|id| format!("https://www.youtube.com/watch?v={}", id))
        .collect();

    println!("Found {} videos in playlist", urls.len());

    if urls.is_empty() {
        return Err("No URLs found in playlist".into());
    }

    Ok(urls)
}

async fn download_song(
    yt_dlp_path: PathBuf,
    url: String,
    audio_format: AudioFormat,
    download_dir: String,
    download_thumbnail: bool,
    ffmpeg_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = vec![
        "--ffmpeg-location",
        ffmpeg_path.to_str().unwrap(),
        "-x",
        "--audio-format",
        audio_format.as_str(),
        "--audio-quality",
        "0",
    ];

    if download_thumbnail {
        args.push("--embed-thumbnail");
    }

    args.extend_from_slice(&["--postprocessor-args", "-ar 48000 -ac 2 -b:a 320k"]);

    args.extend_from_slice(&[
        "-P",
        &download_dir,
        "--no-check-certificates",
        "--ignore-errors",
        "--print",
        "after_move:%(filepath)s",
        "--output",
        "%(title)s.%(ext)s",
        "--parse-metadata",
        "%(uploader)s:%(artist)s",
        "--replace-in-metadata",
        "title",
        "^.*? - ",
        "",
        "--replace-in-metadata",
        "title",
        "\\s*\\([^)]*\\)",
        "",
        "--add-metadata",
        &url,
    ]);

    let output = Command::new(&yt_dlp_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    if output.status.success() {
        if let Ok(filepath) = String::from_utf8(output.stdout) {
            let filepath = filepath.trim();
            if !filepath.is_empty() {
                let path = PathBuf::from(filepath);
                if let Some(dir) = path.parent() {
                    if let Some(filename) = path.file_name() {
                        if let Some(filename_str) = filename.to_str() {
                            let re_artist = Regex::new(r"^.*? - ").unwrap();
                            let mut new_filename = re_artist.replace(filename_str, "").to_string();

                            let re_prod = Regex::new(r"\s*\([^)]*\)").unwrap();
                            new_filename = re_prod.replace_all(&new_filename, "").to_string();

                            let re_spaces = Regex::new(r"\s+\.").unwrap();
                            new_filename = re_spaces.replace_all(&new_filename, ".").to_string();

                            let new_path = dir.join(&new_filename);

                            if path != new_path {
                                fs::rename(&path, &new_path)?;
                                fs::rename(&new_path, &new_path)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        println!("Failed to download {}: {}", url, error);
        Err(error.into())
    }
}

#[derive(Parser)]
#[command(name = "ytpd")]
#[command(about = "Youtube Music Downloader")]
struct Cli {
    url: Option<String>,
}

#[derive(Clone, Copy)]
enum AudioFormat {
    Mp3,
    Wav,
    M4a,
    Aac,
    Flac,
}

impl AudioFormat {
    fn as_str(&self) -> &'static str {
        match self {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Wav => "wav",
            AudioFormat::M4a => "m4a",
            AudioFormat::Aac => "aac",
            AudioFormat::Flac => "flac",
        }
    }
}

#[tokio::main]
async fn main() {
    let setup_config = match check_dependencies().await {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Setup failed: {}", e);
            return;
        }
    };

    let yt_dlp_path = setup_config.yt_dlp_path;

    let download_dir = get_download_directory().expect("Failed to setup download directory");

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
        0 => AudioFormat::Mp3,
        1 => AudioFormat::Wav,
        2 => AudioFormat::M4a,
        3 => AudioFormat::Aac,
        4 => AudioFormat::Flac,
        _ => AudioFormat::Mp3,
    };

    let download_thumbnail = should_download_thumbail();

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} Downloading... {wide_msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(Duration::from_millis(100));

    if selection == 0 {
        let result = download_song(
            yt_dlp_path,
            url,
            audio_format,
            download_dir,
            download_thumbnail,
            setup_config.ffmpeg_path.unwrap(),
        )
        .await;
        spinner.finish_and_clear();
        match result {
            Ok(_) => println!("Download completed!"),
            Err(e) => println!("Download failed: {}", e),
        }
    } else {
        println!("Fetching playlist information...");
        match get_playlist_urls(yt_dlp_path.clone(), &url).await {
            Ok(playlist_urls) => {
                println!("Starting download of {} videos...", playlist_urls.len());
                let mut handles = vec![];

                let semaphore = Arc::new(Semaphore::new(44));

                let ffmpeg_path = setup_config.ffmpeg_path.unwrap().clone();

                for url in playlist_urls.iter() {
                    let yt_dlp_path = yt_dlp_path.clone();
                    let url = url.to_string();
                    let download_dir = download_dir.clone();
                    let sem = semaphore.clone();
                    let ffmpeg_path = ffmpeg_path.clone();

                    let handle = task::spawn(async move {
                        let _permit = sem.acquire().await.unwrap();
                        download_song(
                            yt_dlp_path,
                            url,
                            audio_format,
                            download_dir,
                            download_thumbnail,
                            ffmpeg_path,
                        )
                        .await
                    });
                    handles.push(handle);
                }

                let results = join_all(handles).await;
                spinner.finish_and_clear();

                let mut success_count = 0;
                let mut failure_count = 0;
                for result in results {
                    match result {
                        Ok(Ok(_)) => success_count += 1,
                        _ => failure_count += 1,
                    }
                }

                println!(
                    "Playlist download completed! Successful: {}, Failed: {}",
                    success_count, failure_count
                );
            }
            Err(e) => {
                spinner.finish_and_clear();
                println!("Failed to get playlist URLs: {}", e);
            }
        }
    }
}
