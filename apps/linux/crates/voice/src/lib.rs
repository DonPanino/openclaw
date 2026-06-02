use serde::{Deserialize, Serialize};
use tokio::process::Command;

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

pub struct VoiceRuntime {
    pub config: VoiceWakeConfig,
    ptt: PttState,
}

impl VoiceRuntime {
    pub fn new(config: VoiceWakeConfig) -> Self {
        Self {
            config,
            ptt: PttState::Idle,
        }
    }

    pub fn start_ptt(&mut self) {
        self.ptt = PttState::Recording;
        tracing::info!("PTT start");
    }

    pub fn stop_ptt(&mut self) -> Option<String> {
        if self.ptt == PttState::Recording {
            self.ptt = PttState::Idle;
            tracing::info!("PTT stop");
            return Some(String::new());
        }
        None
    }

    pub fn ptt_state(&self) -> PttState {
        self.ptt
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
}

pub async fn collect_voice_diagnostics(
    voice_wake_enabled: bool,
    talk_enabled: bool,
    ptt: PttState,
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
    }
}

async fn executable_on_path(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod voice_diag_tests {
    use super::{collect_voice_diagnostics, PttState};

    #[tokio::test]
    async fn diagnostics_include_ptt_state_label() {
        let diag = collect_voice_diagnostics(true, false, PttState::Recording).await;
        assert!(diag.voice_wake_enabled);
        assert!(!diag.talk_enabled);
        assert_eq!(diag.ptt_state, "recording");
    }
}

pub async fn synthesize_talk_stub(text: &str) -> Vec<u8> {
    tracing::info!("talk TTS stub for: {text}");
    Vec::new()
}
