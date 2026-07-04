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
