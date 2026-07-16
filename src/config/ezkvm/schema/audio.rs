use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum AudioSchema {
    Alsa { alsa: AlsaSchema },
    PulseAudio { pulse_audio: PulseAudioSchema },
    PipeWire { pipe_wire: PipeWireSchema },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlsaSchema {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PulseAudioSchema {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PipeWireSchema {}
