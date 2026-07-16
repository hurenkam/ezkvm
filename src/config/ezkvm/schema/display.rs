use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum DisplaySchema {
    Vnc { vnc: VncSchema },
    Spice { spice: SpiceSchema },
    EglHeadless { egl_headless: EglHeadlessSchema },
    LookingGlass { looking_glass: LookingGlassSchema },
    Gtk { gtk: GtkSchema },
    Sdl { sdl: SdlSchema },
}

#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct LookingGlassSchema {
    port: u16,
    pulisten: String,
    disable_ticketing: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct VncSchema {
    port: u16,
    listen: String,
    #[serde(default)]
    gl_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    socket_path: Option<String>,
    #[serde(default)]
    password_auth: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct SpiceSchema {
    port: u16,
    listen: String,
    disable_ticketing: bool,
    #[serde(default)]
    gl_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tls_port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tls_ciphers: Option<String>,
    #[serde(default)]
    seamless_migration: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EglHeadlessSchema {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GtkSchema {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SdlSchema {}
