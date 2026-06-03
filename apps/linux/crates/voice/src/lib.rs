use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use thiserror::Error;
use tokio::process::Command as AsyncCommand;

#[derive(Error, Debug)]
pub enum PttError {
    #[error("no recorder on PATH (install pipewire-pulse/pw-record or pulseaudio-utils/parecord)")]
    NoRecorder,
    #[error("PTT already recording")]
    AlreadyRecording,
    #[error("PTT not recording")]
    NotRecording,
    #[error("recorder failed: {0}")]
    Recorder(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoiceWakeConfig {
    pub enabled: bool,
    #[serde(default)]
    pub talk_enabled: bool,
    #[serde(default)]
    pub phrases: Vec<String>,
    #[serde(default = "default_locale")]
    pub locale: String,
}

fn default_locale() -> String {
    "en-US".into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PttState {
    Idle,
    Recording,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PttStopResult {
    pub transcript: String,
    pub audio_base64: Option<String>,
}

/// Prefer PipeWire `pw-record`, else PulseAudio `parecord`.
pub fn select_ptt_recorder(pw_record: bool, parecord: bool) -> Option<&'static str> {
    if pw_record {
        Some("pw-record")
    } else if parecord {
        Some("parecord")
    } else {
        None
    }
}

pub struct PttRecorder {
    child: Child,
    output_path: PathBuf,
    bin: &'static str,
}

impl PttRecorder {
    pub fn start() -> Result<Self, PttError> {
        let pw = std::path::Path::new("/usr/bin/pw-record").exists()
            || which_sync("pw-record");
        let pa = which_sync("parecord");
        let bin = select_ptt_recorder(pw, pa).ok_or(PttError::NoRecorder)?;
        let output_path = temp_wav_path();
        let child = spawn_recorder(bin, &output_path).map_err(PttError::Recorder)?;
        Ok(Self {
            child,
            output_path,
            bin,
        })
    }

    pub fn stop(mut self) -> Result<Vec<u8>, PttError> {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let bytes = std::fs::read(&self.output_path)
            .map_err(|e| PttError::Recorder(format!("read {}: {e}", self.output_path.display())))?;
        let _ = std::fs::remove_file(&self.output_path);
        if bytes.is_empty() {
            return Err(PttError::Recorder("empty WAV capture".into()));
        }
        Ok(bytes)
    }

    pub fn cancel(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.output_path);
    }

    pub fn recorder_bin(&self) -> &'static str {
        self.bin
    }
}

fn temp_wav_path() -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("openclaw-ptt-{}.wav", uuid::Uuid::new_v4()));
    path
}

fn spawn_recorder(bin: &str, output_path: &PathBuf) -> Result<Child, String> {
    let path_str = output_path
        .to_str()
        .ok_or_else(|| "invalid temp path".to_string())?;
    let child = match bin {
        "pw-record" => Command::new("pw-record")
            .args(["-q", path_str])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn(),
        "parecord" => Command::new("parecord")
            .args([format!("--file={path_str}")])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn(),
        other => return Err(format!("unknown recorder {other}")),
    }
    .map_err(|e| format!("spawn {bin}: {e}"))?;
    Ok(child)
}

fn which_sync(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn wav_bytes_to_base64(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

pub struct VoiceRuntime {
    pub config: VoiceWakeConfig,
    recorder: Option<PttRecorder>,
}

impl VoiceRuntime {
    pub fn new(config: VoiceWakeConfig) -> Self {
        Self {
            config,
            recorder: None,
        }
    }

    pub fn start_ptt(&mut self) -> Result<(), PttError> {
        if self.recorder.is_some() {
            return Err(PttError::AlreadyRecording);
        }
        let recorder = PttRecorder::start()?;
        tracing::info!("PTT start ({})", recorder.recorder_bin());
        self.recorder = Some(recorder);
        Ok(())
    }

    pub fn stop_ptt(&mut self) -> PttStopResult {
        let Some(recorder) = self.recorder.take() else {
            tracing::info!("PTT stop (idle)");
            return PttStopResult::default();
        };
        tracing::info!("PTT stop");
        match recorder.stop() {
            Ok(bytes) => PttStopResult {
                transcript: String::new(),
                audio_base64: Some(wav_bytes_to_base64(&bytes)),
            },
            Err(err) => {
                tracing::warn!("PTT stop failed: {err}");
                PttStopResult::default()
            }
        }
    }

    pub fn cancel_ptt(&mut self) {
        if let Some(recorder) = self.recorder.take() {
            tracing::info!("PTT cancel");
            recorder.cancel();
        }
    }

    pub fn ptt_once(&mut self) -> PttStopResult {
        if let Err(err) = self.start_ptt() {
            tracing::warn!("PTT once start failed: {err}");
            return PttStopResult::default();
        }
        std::thread::sleep(Duration::from_millis(400));
        self.stop_ptt()
    }

    pub fn ptt_state(&self) -> PttState {
        if self.recorder.is_some() {
            PttState::Recording
        } else {
            PttState::Idle
        }
    }

    pub async fn process_wake_audio(&self, _pcm: &[i8]) -> Option<String> {
        if !self.config.enabled {
            return None;
        }
        None
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct VoiceDiagnostics {
    pub voice_wake_enabled: bool,
    pub talk_enabled: bool,
    pub pw_record: bool,
    pub parecord: bool,
    pub ptt_state: String,
    pub ptt_recorder_active: bool,
}

pub async fn collect_voice_diagnostics(
    voice_wake_enabled: bool,
    talk_enabled: bool,
    ptt: PttState,
    ptt_recorder_active: bool,
) -> VoiceDiagnostics {
    VoiceDiagnostics {
        voice_wake_enabled,
        talk_enabled,
        pw_record: executable_on_path("pw-record").await,
        parecord: executable_on_path("parecord").await,
        ptt_state: match ptt {
            PttState::Idle => "idle",
            PttState::Recording => "recording",
        }
        .into(),
        ptt_recorder_active,
    }
}

async fn executable_on_path(name: &str) -> bool {
    AsyncCommand::new("which")
        .arg(name)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod ptt_tests {
    use super::*;

    #[test]
    fn select_recorder_prefers_pw_record() {
        assert_eq!(select_ptt_recorder(true, true), Some("pw-record"));
        assert_eq!(select_ptt_recorder(true, false), Some("pw-record"));
        assert_eq!(select_ptt_recorder(false, true), Some("parecord"));
        assert_eq!(select_ptt_recorder(false, false), None);
    }

    #[test]
    fn wav_base64_roundtrip() {
        assert_eq!(wav_bytes_to_base64(b"RIFF"), "UklGRg==");
    }
}

#[cfg(test)]
mod voice_diag_tests {
    use super::{collect_voice_diagnostics, PttState};

    #[tokio::test]
    async fn diagnostics_include_ptt_state_label() {
        let diag = collect_voice_diagnostics(true, false, PttState::Recording, true).await;
        assert!(diag.voice_wake_enabled);
        assert!(!diag.talk_enabled);
        assert_eq!(diag.ptt_state, "recording");
        assert!(diag.ptt_recorder_active);
    }
}

pub async fn synthesize_talk_stub(text: &str) -> Vec<u8> {
    tracing::info!("talk TTS stub for: {text}");
    Vec::new()
}
