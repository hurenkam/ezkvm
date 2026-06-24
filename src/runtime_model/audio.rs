use std::{fmt, sync::Arc};

use serde::{Deserialize, Serialize};

/// Config-level audio configuration for the virtual machine.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Audio {
    pub backend: AudioBackend,
    pub controller: AudioController,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioBackend {
    None,
    Alsa,
    PulseAudio,
    #[default]
    PipeWire,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioController {
    #[default]
    Ich9IntelHda,
    Ac97,
}

pub trait AudioApi: fmt::Display {
    fn config(&self) -> &Audio;
    fn qemu_args(&self) -> Vec<String>;
}

pub struct AudioModelBuilder {}

impl AudioModelBuilder {
    pub fn build(audio: Audio) -> Arc<dyn AudioApi> {
        Arc::new(AudioModel { audio })
    }
}

struct AudioModel {
    audio: Audio,
}

impl AudioApi for AudioModel {
    fn config(&self) -> &Audio {
        &self.audio
    }

    fn qemu_args(&self) -> Vec<String> {
        let backend_id = "audio0";
        let backend_str = match &self.audio.backend {
            AudioBackend::None => return vec![],
            AudioBackend::Alsa => "alsa",
            AudioBackend::PulseAudio => "pa",
            AudioBackend::PipeWire => "pipewire",
        };

        let mut args = vec![
            "-audiodev".to_string(),
            format!("{},id={}", backend_str, backend_id),
        ];

        match &self.audio.controller {
            AudioController::Ich9IntelHda => {
                args.extend([
                    "-device".to_string(),
                    "ich9-intel-hda,id=sound0".to_string(),
                    "-device".to_string(),
                    format!(
                        "hda-duplex,id=sound0-codec0,bus=sound0.0,cad=0,audiodev={}",
                        backend_id
                    ),
                ]);
            }
            AudioController::Ac97 => {
                args.extend([
                    "-device".to_string(),
                    format!("AC97,audiodev={}", backend_id),
                ]);
            }
        }

        args
    }
}

impl fmt::Display for AudioModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Audio ({:?} + {:?})",
            self.audio.backend, self.audio.controller
        )
    }
}
