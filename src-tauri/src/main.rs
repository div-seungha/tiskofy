use std::{env, fs::File, io::{self, Cursor}, path::PathBuf};
use tauri::{command, AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;
use reqwest;
use zip::ZipArchive;
use tokio::process::Command;


fn sanitize_filename(title: &str) -> String {
    title
        .replace(|c: char| !c.is_ascii_alphanumeric() && c != ' ' && c != '-' && c != '_', "")
}

#[command]
async fn download_mp3(url: String, app: AppHandle) -> String {
    eprintln!("[1] Start download_mp3");

    if !(url.contains("youtube.com") || url.contains("youtu.be")) {
        eprintln!("[Error] Invalid YouTube URL: {}", url);
        return "invalid URL".to_string();
    }

    // yt-dlp
    let yt_dlp_path = match ensure_yt_dlp_exists().await {
        Ok(path) => {
            eprintln!("[2] yt-dlp path resolved: {:?}", path);
            path
        },
        Err(e) => {
            eprintln!("[Error] Failed to ensure yt-dlp: {}", e);
            return format!("Error ensuring yt-dlp: {}", e);
        }
    };

    // ffmpeg
    let ffmpeg_path = ensure_ffmpeg_exists().await.unwrap_or(None);

    let mut cmd = Command::new(&yt_dlp_path);
    cmd.args(["-x", "--audio-format", "mp3"]);

    if let Some(ref path) = ffmpeg_path {
        if let Some(path_str) = path.to_str() {
            eprintln!("[Debug] --ffmpeg-location = {}", path_str);
            cmd.args(["--ffmpeg-location", path_str]);
        } else {
            eprintln!("[Warning] Cannot convert ffmpeg path to str");
        }
    } else {
        eprintln!("[Warning] ffmpeg not found, skipping --ffmpeg-location");
    };

    // Get video title
    let title_output = Command::new(&yt_dlp_path)
        .args(["--print", "title", &url])
        .output()
        .await;

    let video_title = match title_output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("[3] yt-dlp stdout: {}", stdout);
            eprintln!("[3] yt-dlp stderr: {}", stderr);
            if output.status.success() {
                stdout
            } else {
                return format!("Error: yt-dlp failed to get title: {}", stderr);
            }
        },
        Err(e) => {
            return format!("Error: Failed to execute yt-dlp: {}", e);
        }
    };

    let sanitized_title = sanitize_filename(&video_title);
    eprintln!("[4] Sanitized title: {}", sanitized_title);

    // Default downloads directory
    let downloads_folder = app
        .path()
        .download_dir()
        .unwrap_or_else(|_| PathBuf::from("~/Downloads"));

    eprintln!("[5] Default download folder: {:?}", downloads_folder);

    // Open save file dialog
    let folder = app
        .dialog()
        .file()
        .set_directory(downloads_folder)
        .set_file_name(&video_title)
        .blocking_save_file();

    let folder_path = match folder {
        Some(path) => {
            let path_str = path.to_string();
            eprintln!("[6] User selected folder: {}", path_str);
            path_str
        },
        None => {
            eprintln!("[Info] Save dialog canceled");
            return "canceled".to_string();
        }
    };

    let output_path = format!("{}/{}.mp3", folder_path, sanitized_title);
    eprintln!("[7] Output path: {}", output_path);


    cmd.args(["-o", &output_path, &url]);

    let output = cmd.output().await;

    if let Some(path) = ffmpeg_path {
        if let Some(path_str) = path.to_str() {
            eprintln!("[Debug] Testing ffmpeg executable...");
            let ffmpeg_test = Command::new(path_str)
                .arg("-version")
                .output()
                .await;

            match ffmpeg_test {
                Ok(output) => {
                    eprintln!("[Debug] ffmpeg version: {}", String::from_utf8_lossy(&output.stdout));
                    cmd.args(["--ffmpeg-location", path_str]);
                },
                Err(e) => {
                    eprintln!("[Error] ffmpeg not executable: {}", e);
                }
            }
        }
    };

    if let Ok(ref r) = output {
        std::fs::write("/tmp/yt-dlp-stdout.txt", &r.stdout).ok();
        std::fs::write("/tmp/yt-dlp-stderr.txt", &r.stderr).ok();
    };

    match output {
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.stderr);
            let stdout = String::from_utf8_lossy(&result.stdout);
            eprintln!("[8] yt-dlp stdout: {}", stdout);
            eprintln!("[8] yt-dlp stderr: {}", stderr);

            if result.status.success() {
                eprintln!("[9] Download completed successfully");
                format!("Ok: path= {}", folder_path)
            } else {
                format!("Error: yt-dlp failed: {}", stderr)
            }
        }
        Err(e) => {
            eprintln!("[Error] Failed to run yt-dlp for download: {}", e);
            format!("Error: Failed to run yt-dlp: {}", e)
        }
    }
}

async fn ensure_yt_dlp_exists() -> io::Result<PathBuf> {
    let app_dir = match env::current_exe()?.parent() {
        Some(dir) => dir.to_path_buf(),
        None => {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to determine app directory",
            ))
        }
    };

    let yt_dlp_path = app_dir.join("yt-dlp");


    if !yt_dlp_path.exists() {
        eprintln!("[ensure] yt-dlp not found, downloading...");

        let download_status = Command::new("curl")
            .args([
                "-L",
                "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp",
                "-o",
                yt_dlp_path.to_str().unwrap(),
            ])
            .status()
            .await;

        if let Err(e) = download_status {
            return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to run curl: {}", e)));
        }

        if !download_status.unwrap().success() {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to download yt-dlp"));
        }

        let chmod_status = Command::new("chmod")
            .args(["+x", yt_dlp_path.to_str().unwrap()])
            .status()
            .await;

        if let Err(e) = chmod_status {
            return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to chmod: {}", e)));
        }

        if !chmod_status.unwrap().success() {
            return Err(io::Error::new(io::ErrorKind::Other, "chmod failed for yt-dlp"));
        }

        eprintln!("[ensure] yt-dlp downloaded and made executable");
    }

    Ok(yt_dlp_path)
}


pub async fn ensure_ffmpeg_exists() -> io::Result<Option<PathBuf>> {
    let app_dir = env::current_exe()?.parent().unwrap().to_path_buf();
    let ffmpeg_path = app_dir.join("ffmpeg");

    if ffmpeg_path.exists() {
        return Ok(Some(ffmpeg_path));
    }

    eprintln!("[ensure] ffmpeg not found, downloading zip...");

    let zip_url = "https://evermeet.cx/ffmpeg/ffmpeg-118896-g9f0970ee35.zip";
    let response = reqwest::get(zip_url).await.map_err(|e| {
        io::Error::new(io::ErrorKind::Other, format!("Download failed: {}", e))
    })?;
    let bytes = response.bytes().await.map_err(|e| {
        io::Error::new(io::ErrorKind::Other, format!("Read failed: {}", e))
    })?;

    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader).map_err(|e| {
        io::Error::new(io::ErrorKind::Other, format!("Zip parse failed: {}", e))
    })?;

    let mut extracted = false;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = ffmpeg_path.clone();

        if file.name().ends_with("ffmpeg") {
            let mut outfile = File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
            extracted = true;
            break;
        }
    }

    if !extracted {
        return Err(io::Error::new(io::ErrorKind::Other, "ffmpeg not found in zip"));
    }

    // chmod +x
    Command::new("chmod")
        .args(["+x", ffmpeg_path.to_str().unwrap()])
        .status()
        .await?;

    eprintln!("[ensure] ffmpeg downloaded, extracted, and made executable");
    Ok(Some(ffmpeg_path))
}



fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![download_mp3])
        .run(tauri::generate_context!())
        .expect("Error");
}
