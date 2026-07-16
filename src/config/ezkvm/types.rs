use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EglHeadlessSchema {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GtkSchema {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SdlSchema {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlsaSchema {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PulseAudioSchema {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PipeWireSchema {}
