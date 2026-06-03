mod portal;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::fs;
use tokio::process::Command;

/// Gateway/CLI media payloads use standard base64 (no line breaks).
pub fn bytes_to_base64(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

#[derive(Error, Debug)]
pub enum CaptureError {
    #[error("portal unavailable: {0}")]
    Portal(String),
    #[error("capture failed: {0}")]
    Failed(String),
    #[error("not implemented on this session: {0}")]
    NotImplemented(String),
}

/// Screen snapshot: xdg-desktop-portal, then `grim`, `gnome-screenshot`, ImageMagick.
pub async fn screen_snapshot() -> Result<Vec<u8>, CaptureError> {
    if let Ok(bytes) = portal::portal_screen_snapshot().await {
        if !bytes.is_empty() {
            return Ok(bytes);
        }
    }
    if let Ok(bytes) = run_capture_command("grim", &["-"]).await {
        if !bytes.is_empty() {
            return Ok(bytes);
        }
    }
    if let Ok(bytes) = run_capture_command("gnome-screenshot", &["-f", "-"]).await {
        if !bytes.is_empty() {
            return Ok(bytes);
        }
    }
    if let Ok(bytes) = run_capture_command("import", &["-window", "root", "png:-"]).await {
        if !bytes.is_empty() {
            return Ok(bytes);
        }
    }
    Err(CaptureError::Failed(
        "no screen capture tool found (install grim, gnome-screenshot, or ImageMagick)".into(),
    ))
}

/// Short camera burst: prefer `ffmpeg` (~1.5s), else multiple `fswebcam` frames (last frame wins).
pub async fn camera_clip() -> Result<Vec<u8>, CaptureError> {
    if has_bin("ffmpeg").await {
        if let Ok(bytes) = ffmpeg_camera_clip_jpeg().await {
            if !bytes.is_empty() {
                return Ok(bytes);
            }
        }
    }
    if has_bin("fswebcam").await {
        if let Ok(bytes) = fswebcam_burst_jpeg(3).await {
            if !bytes.is_empty() {
                return Ok(bytes);
            }
        }
    }
    Err(CaptureError::Failed(
        "camera.clip: install ffmpeg (v4l2) or fswebcam for a short JPEG burst".into(),
    ))
}

async fn ffmpeg_camera_clip_jpeg() -> Result<Vec<u8>, CaptureError> {
    let duration_secs = "1.5";
    for input in ["/dev/video0", "default"] {
        let output = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "v4l2",
                "-i",
                input,
                "-t",
                duration_secs,
                "-frames:v",
                "1",
                "-f",
                "image2pipe",
                "-vcodec",
                "mjpeg",
                "pipe:1",
            ])
            .output()
            .await
            .map_err(|e| CaptureError::Failed(format!("ffmpeg: {e}")))?;
        if output.status.success() && !output.stdout.is_empty() {
            return Ok(output.stdout);
        }
    }
    Err(CaptureError::Failed("ffmpeg camera clip produced no frames".into()))
}

async fn fswebcam_burst_jpeg(frames: u8) -> Result<Vec<u8>, CaptureError> {
    let mut last: Option<Vec<u8>> = None;
    for _ in 0..frames {
        if let Ok(bytes) = run_capture_command("fswebcam", &["-q", "-r", "1280x720", "--no-banner", "-"]).await
        {
            if !bytes.is_empty() {
                last = Some(bytes);
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    last.ok_or_else(|| CaptureError::Failed("fswebcam burst produced no frames".into()))
}

/// Short screen clip for gateway `screen.record` (`nodes-screen.ts` payload shape).
pub struct ScreenRecord {
    pub bytes: Vec<u8>,
    pub duration_ms: u64,
    pub has_audio: bool,
    /// `webm` or `mp4`
    pub format: &'static str,
}

const RECORD_SECS: &str = "2";
const RECORD_DURATION_MS: u64 = 2000;

/// ~2s clip: `wf-recorder` (Wayland), else `ffmpeg` x11grab (X11), else not implemented.
pub async fn screen_record() -> Result<ScreenRecord, CaptureError> {
    if has_bin("wf-recorder").await {
        if let Ok(rec) = wf_recorder_clip().await {
            return Ok(rec);
        }
    }
    if has_bin("ffmpeg").await {
        if let Ok(rec) = ffmpeg_x11grab_clip().await {
            return Ok(rec);
        }
    }
    Err(CaptureError::NotImplemented(
        "screen.record: install wf-recorder (Wayland) or ffmpeg with x11grab (X11); use screen.snapshot otherwise"
            .into(),
    ))
}

fn temp_media_path(ext: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("openclaw-screen-record-{ts}.{ext}"))
}

async fn run_wf_recorder(path: &PathBuf, with_audio: bool) -> Result<std::process::ExitStatus, CaptureError> {
    let path_str = path.to_string_lossy();
    let mut args = vec![
        "-f",
        path_str.as_ref(),
        "-d",
        RECORD_SECS,
        "--log-level",
        "error",
    ];
    if with_audio {
        args.push("--audio");
    }
    let output = Command::new("wf-recorder")
        .args(&args)
        .output()
        .await
        .map_err(|e| CaptureError::Failed(format!("wf-recorder: {e}")))?;
    Ok(output.status)
}

async fn wf_recorder_clip() -> Result<ScreenRecord, CaptureError> {
    let path = temp_media_path("webm");
    let mut with_audio = true;
    let mut status = run_wf_recorder(&path, true).await?;
    if !status.success() {
        with_audio = false;
        let _ = fs::remove_file(&path).await;
        status = run_wf_recorder(&path, false).await?;
    }
    if !status.success() {
        let _ = fs::remove_file(&path).await;
        return Err(CaptureError::Failed(format!("wf-recorder exit {:?}", status.code())));
    }
    let bytes = fs::read(&path).await.map_err(|e| CaptureError::Failed(e.to_string()))?;
    let _ = fs::remove_file(&path).await;
    if bytes.is_empty() {
        return Err(CaptureError::Failed("wf-recorder produced empty file".into()));
    }
    Ok(ScreenRecord {
        bytes,
        duration_ms: RECORD_DURATION_MS,
        has_audio: with_audio,
        format: "webm",
    })
}

async fn ffmpeg_x11grab_clip() -> Result<ScreenRecord, CaptureError> {
    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0.0".into());
    let path = temp_media_path("mp4");
    let path_str = path.to_string_lossy();
    let output = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "x11grab",
            "-framerate",
            "25",
            "-i",
            &display,
            "-t",
            RECORD_SECS,
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-movflags",
            "+faststart",
            path_str.as_ref(),
        ])
        .output()
        .await
        .map_err(|e| CaptureError::Failed(format!("ffmpeg: {e}")))?;
    if !output.status.success() {
        let _ = fs::remove_file(&path).await;
        return Err(CaptureError::Failed(format!(
            "ffmpeg x11grab exit {:?}",
            output.status.code()
        )));
    }
    let bytes = fs::read(&path).await.map_err(|e| CaptureError::Failed(e.to_string()))?;
    let _ = fs::remove_file(&path).await;
    if bytes.is_empty() {
        return Err(CaptureError::Failed("ffmpeg x11grab produced empty file".into()));
    }
    Ok(ScreenRecord {
        bytes,
        duration_ms: RECORD_DURATION_MS,
        has_audio: false,
        format: "mp4",
    })
}

pub async fn camera_snap() -> Result<Vec<u8>, CaptureError> {
    if let Ok(bytes) = run_capture_command("fswebcam", &["-q", "-r", "1280x720", "--no-banner", "-"]).await {
        if !bytes.is_empty() {
            return Ok(bytes);
        }
    }
    Err(CaptureError::Failed(
        "no camera tool found (install fswebcam or use portal capture)".into(),
    ))
}

async fn run_capture_command(bin: &str, args: &[&str]) -> Result<Vec<u8>, CaptureError> {
    let output = Command::new(bin)
        .args(args)
        .output()
        .await
        .map_err(|e| CaptureError::Failed(format!("{bin}: {e}")))?;
    if output.status.success() && !output.stdout.is_empty() {
        return Ok(output.stdout);
    }
    Err(CaptureError::Failed(format!(
        "{bin} exit {:?}",
        output.status.code()
    )))
}

/// `location.get` payload per docs/nodes/location-command.md
pub async fn location_get() -> Result<Value, CaptureError> {
    if let Some((lat, lon, accuracy)) = location_from_gpspipe().await {
        return Ok(location_payload(lat, lon, accuracy, "gps"));
    }
    if let Some((lat, lon, accuracy)) = location_from_geoclue_demo().await {
        return Ok(location_payload(lat, lon, accuracy, "wifi"));
    }
    Err(CaptureError::NotImplemented(
        "install gpsd+gpspipe or geoclue2 demo tools for location.get".into(),
    ))
}

fn location_payload(lat: f64, lon: f64, accuracy_meters: f64, source: &str) -> Value {
    json!({
        "lat": lat,
        "lon": lon,
        "accuracyMeters": accuracy_meters,
        "source": source,
        "isPrecise": accuracy_meters <= 50.0,
    })
}

async fn location_from_gpspipe() -> Option<(f64, f64, f64)> {
    let output = Command::new("gpspipe")
        .args(["-w", "-n", "8"])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        if !line.contains("\"class\":\"tpv\"") && !line.contains("\"class\": \"tpv\"") {
            continue;
        }
        let parsed: Value = serde_json::from_str(line).ok()?;
        let lat = parsed.get("lat")?.as_f64()?;
        let lon = parsed.get("lon")?.as_f64()?;
        if lat == 0.0 && lon == 0.0 {
            continue;
        }
        let accuracy = parsed
            .get("eph")
            .and_then(|v| v.as_f64())
            .unwrap_or(25.0);
        return Some((lat, lon, accuracy));
    }
    None
}

async fn location_from_geoclue_demo() -> Option<(f64, f64, f64)> {
    let output = Command::new("busctl")
        .args([
            "--system",
            "get-property",
            "org.freedesktop.GeoClue2",
            "/org/freedesktop/GeoClue2/Location",
            "org.freedesktop.GeoClue2.Location",
            "Latitude",
        ])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lat = parse_busctl_double(&stdout)?;
    let lon_out = Command::new("busctl")
        .args([
            "--system",
            "get-property",
            "org.freedesktop.GeoClue2",
            "/org/freedesktop/GeoClue2/Location",
            "org.freedesktop.GeoClue2.Location",
            "Longitude",
        ])
        .output()
        .await
        .ok()?;
    let lon = parse_busctl_double(&String::from_utf8_lossy(&lon_out.stdout))?;
    let accuracy_out = Command::new("busctl")
        .args([
            "--system",
            "get-property",
            "org.freedesktop.GeoClue2",
            "/org/freedesktop/GeoClue2/Location",
            "org.freedesktop.GeoClue2.Location",
            "Accuracy",
        ])
        .output()
        .await
        .ok()?;
    let accuracy = parse_busctl_double(&String::from_utf8_lossy(&accuracy_out.stdout)).unwrap_or(100.0);
    Some((lat, lon, accuracy))
}

fn parse_busctl_double(raw: &str) -> Option<f64> {
    let token = raw.split_whitespace().last()?;
    token.parse().ok()
}

pub async fn list_cameras() -> Result<Vec<String>, CaptureError> {
    Ok(vec!["default".into()])
}

async fn has_bin(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Best-effort probe of capture/location tools (permissions tab).
pub async fn capture_diagnostics() -> Value {
    let portal_session = zbus::Connection::session().await.is_ok();
    json!({
        "portalSession": portal_session,
        "grim": has_bin("grim").await,
        "gnomeScreenshot": has_bin("gnome-screenshot").await,
        "imagemagick": has_bin("import").await,
        "fswebcam": has_bin("fswebcam").await,
        "ffmpeg": has_bin("ffmpeg").await,
        "wfRecorder": has_bin("wf-recorder").await,
        "gpspipe": has_bin("gpspipe").await,
        "geoclueBusctl": has_bin("busctl").await,
    })
}

#[cfg(test)]
mod base64_tests {
    use super::*;

    #[test]
    fn bytes_to_base64_roundtrip() {
        assert_eq!(bytes_to_base64(b"hi"), "aGk=");
        assert_eq!(bytes_to_base64(&[]), "");
    }
}
