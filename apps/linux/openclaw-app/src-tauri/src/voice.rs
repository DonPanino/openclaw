use openclaw_kit::gateway_config::GatewayConnectionSettings;
use openclaw_voice::{
    collect_voice_diagnostics, PttError, PttStopResult, VoiceDiagnostics, VoiceRuntime,
    VoiceWakeConfig,
};
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

    /// Persists linux-app voice settings locally. Gateway `voicewake.set` is invoked from
    /// `save_voice_settings` when an operator session is connected (macOS global sync parity).
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
        rt.start_ptt().map_err(ptt_error_message)
    }

    pub fn stop_ptt(&self) -> Result<PttStopResult, String> {
        let mut rt = self.runtime.lock().map_err(|e| e.to_string())?;
        Ok(rt.stop_ptt())
    }

    pub fn cancel_ptt(&self) -> Result<(), String> {
        let mut rt = self.runtime.lock().map_err(|e| e.to_string())?;
        rt.cancel_ptt();
        Ok(())
    }

    pub fn ptt_once(&self) -> Result<PttStopResult, String> {
        let mut rt = self.runtime.lock().map_err(|e| e.to_string())?;
        Ok(rt.ptt_once())
    }

    pub async fn diagnostics(&self) -> Result<VoiceDiagnostics, String> {
        let settings = GatewayConnectionSettings::load();
        let (ptt, active) = self
            .runtime
            .lock()
            .map(|rt| {
                let ptt = rt.ptt_state();
                let active = ptt == openclaw_voice::PttState::Recording;
                (ptt, active)
            })
            .map_err(|e| e.to_string())?;
        Ok(collect_voice_diagnostics(
            settings.voice_wake_enabled,
            settings.talk_enabled,
            ptt,
            active,
        )
        .await)
    }
}

fn ptt_error_message(err: PttError) -> String {
    err.to_string()
}
