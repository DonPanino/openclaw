use openclaw_kit::gateway_config::GatewayConnectionSettings;
use openclaw_voice::{collect_voice_diagnostics, VoiceDiagnostics, VoiceRuntime, VoiceWakeConfig};
use std::sync::Mutex;

pub struct VoiceService {
    runtime: Mutex<VoiceRuntime>,
}

fn config_from_connection(settings: &GatewayConnectionSettings) -> VoiceWakeConfig {
    VoiceWakeConfig {
        enabled: settings.voice_wake_enabled,
        talk_enabled: settings.talk_enabled,
        phrases: if settings.voice_wake_phrases.is_empty() {
            vec!["open claw".into()]
        } else {
            settings.voice_wake_phrases.clone()
        },
        locale: "en-US".into(),
    }
}

impl VoiceService {
    pub fn new() -> Self {
        let config = config_from_connection(&GatewayConnectionSettings::load());
        Self {
            runtime: Mutex::new(VoiceRuntime::new(config)),
        }
    }

    pub fn config(&self) -> VoiceWakeConfig {
        self.runtime
            .lock()
            .map(|rt| rt.config.clone())
            .unwrap_or_default()
    }

    pub fn apply(&self, config: VoiceWakeConfig) -> Result<(), String> {
        let mut settings = GatewayConnectionSettings::load();
        settings.voice_wake_enabled = config.enabled;
        settings.talk_enabled = config.talk_enabled;
        settings.voice_wake_phrases = config.phrases.clone();
        settings.save().map_err(|e| e.to_string())?;
        if let Ok(mut rt) = self.runtime.lock() {
            rt.config = config;
        }
        Ok(())
    }

    pub fn sync_from_connection(&self, settings: &GatewayConnectionSettings) -> Result<(), String> {
        self.apply(config_from_connection(settings))
    }

    pub fn start_ptt(&self) -> Result<(), String> {
        let mut rt = self.runtime.lock().map_err(|e| e.to_string())?;
        rt.start_ptt();
        Ok(())
    }

    pub fn stop_ptt(&self) -> Result<Option<String>, String> {
        let mut rt = self.runtime.lock().map_err(|e| e.to_string())?;
        Ok(rt.stop_ptt())
    }

    pub async fn diagnostics(&self) -> Result<VoiceDiagnostics, String> {
        let settings = GatewayConnectionSettings::load();
        let ptt = self
            .runtime
            .lock()
            .map(|rt| rt.ptt_state())
            .map_err(|e| e.to_string())?;
        Ok(
            collect_voice_diagnostics(
                settings.voice_wake_enabled,
                settings.talk_enabled,
                ptt,
            )
            .await,
        )
    }
}
