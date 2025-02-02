use dialoguer::Select;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

pub struct SetupConfig {
    pub yt_dlp_path: PathBuf,
    pub ffmpeg_path: Option<PathBuf>,
}

pub async fn check_dependencies() -> Result<SetupConfig, Box<dyn std::error::Error>> {
    let ffmpeg_path = match check_ffmpeg() {
        Ok(path) => Some(path),
        Err(_) => {
            println!("⨯ FFmpeg not found");
            show_ffmpeg_install_instructions();
            match install_ffmpeg().await {
                Ok(path) => {
                    println!("✓ FFmpeg installed successfully");
                    Some(path)
                }
                Err(e) => {
                    println!("⨯ Failed to install FFmpeg: {}", e);
                    return Err(e);
                }
            }
        }
    };

    let bin_dir = PathBuf::from("ytpd");
    let yt_dlp_path = if cfg!(windows) {
        bin_dir.join("yt-dlp.exe")
    } else {
        bin_dir.join("yt-dlp")
    };

    let yt_dlp_path = if yt_dlp_path.exists() {
        yt_dlp_path
    } else {
        println!("⨯ yt-dlp not found");
        match ensure_yt_dlp().await {
            Ok(path) => {
                println!("✓ yt-dlp installed successfully");
                path
            }
            Err(e) => {
                println!("⨯ Failed to install yt-dlp: {}", e);
                return Err(e);
            }
        }
    };

    Ok(SetupConfig {
        yt_dlp_path,
        ffmpeg_path,
    })
}

fn check_ffmpeg() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let cmd = if cfg!(windows) { "where" } else { "which" };
    let output = Command::new(cmd).arg("ffmpeg").output()?;

    if !output.status.success() {
        println!("FFmpeg not found in system.");
        return Err("FFmpeg not found".into());
    }

    let test_output = Command::new("ffmpeg").arg("-version").output()?;
    if !test_output.status.success() {
        println!("FFmpeg installation appears to be broken.");
        return Err("FFmpeg verification failed".into());
    }

    let path_str = String::from_utf8(output.stdout)?;
    let path = PathBuf::from(path_str.trim());
    Ok(path)
}

fn show_ffmpeg_install_instructions() {
    println!("\nFFmpeg is required but not found.");
    let options = vec![
        "Automatic installation (Recommended)",
        "Show manual installation instructions",
        "Exit program",
    ];

    let selection = Select::new()
        .with_prompt("What would you like to do?")
        .items(&options)
        .default(0)
        .interact()
        .unwrap();

    match selection {
        0 => (),
        1 => {
            println!("\nFFmpeg Installation Instructions:");
            if cfg!(windows) {
                println!("1. Download FFmpeg from https://ffmpeg.org/download.html");
                println!("2. Extract the archive");
                println!("3. Add FFmpeg bin directory to your PATH environment variable");
            } else if cfg!(target_os = "macos") {
                println!("Option 1 - Using Homebrew (recommended):");
                println!("1. Install Homebrew if not installed: /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"");
                println!("2. Run: brew install ffmpeg");
                println!("\nOption 2 - Manual download:");
                println!("1. Download from https://ffmpeg.org/download.html");
                println!("2. Extract and add to your PATH");
            } else {
                println!("Choose your distribution:");
                let distros = vec!["Ubuntu/Debian", "Fedora", "Arch Linux", "Other"];
                let distro = Select::new()
                    .with_prompt("Select your Linux distribution")
                    .items(&distros)
                    .interact()
                    .unwrap();

                match distro {
                    0 => println!("Run: sudo apt install ffmpeg"),
                    1 => println!("Run: sudo dnf install ffmpeg"),
                    2 => println!("Run: sudo pacman -S ffmpeg"),
                    _ => println!(
                        "Please check your distribution's package manager for FFmpeg installation"
                    ),
                }
            }
            std::process::exit(1);
        }
        2 => {
            println!("Exiting program - FFmpeg is required.");
            std::process::exit(1);
        }
        _ => unreachable!(),
    }
}

async fn install_ffmpeg() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if cfg!(target_os = "windows") {
        install_ffmpeg_windows().await
    } else if cfg!(target_os = "macos") {
        install_ffmpeg_macos().await
    } else {
        install_ffmpeg_linux().await
    }
}

async fn install_ffmpeg_windows() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let bin_dir = PathBuf::from("ytpd");
    fs::create_dir_all(&bin_dir)?;

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} Installing FFmpeg... {wide_msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(Duration::from_millis(100));

    println!("Windows FFmpeg automatic installation not implemented yet.");
    println!("Please install FFmpeg manually from https://ffmpeg.org/download.html");
    Err("Windows FFmpeg installation not implemented yet".into())
}

async fn install_ffmpeg_macos() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} Installing FFmpeg via Homebrew... {wide_msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(Duration::from_millis(100));

    let output = Command::new("brew").args(["install", "ffmpeg"]).output()?;

    spinner.finish_and_clear();

    if output.status.success() {
        check_ffmpeg()
    } else {
        Err("Failed to install FFmpeg via Homebrew".into())
    }
}

async fn install_ffmpeg_linux() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} Installing FFmpeg... {wide_msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(Duration::from_millis(100));

    let (cmd, args) = if Command::new("apt").output().is_ok() {
        ("sudo", vec!["apt", "install", "-y", "ffmpeg"])
    } else if Command::new("dnf").output().is_ok() {
        ("sudo", vec!["dnf", "install", "-y", "ffmpeg"])
    } else if Command::new("pacman").output().is_ok() {
        ("sudo", vec!["pacman", "-S", "--noconfirm", "ffmpeg"])
    } else {
        return Err("No supported package manager found".into());
    };

    let output = Command::new(cmd).args(&args).output()?;

    spinner.finish_and_clear();

    if output.status.success() {
        check_ffmpeg()
    } else {
        Err("Failed to install FFmpeg".into())
    }
}

async fn ensure_yt_dlp() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let bin_dir = PathBuf::from("ytpd");
    fs::create_dir_all(&bin_dir)?;

    let yt_dlp_path = if cfg!(windows) {
        bin_dir.join("yt-dlp.exe")
    } else {
        bin_dir.join("yt-dlp")
    };

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} Installing/Updating yt-dlp... {wide_msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(Duration::from_millis(100));

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

    spinner.finish_and_clear();
    Ok(yt_dlp_path)
}
