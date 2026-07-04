use std::{fmt, sync::Arc};

use derive_getters::Getters;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Display {
    Gtk { gtk: Gtk },
    Sdl { sdl: Sdl },
    Vnc { vnc: Vnc },
    Spice { spice: Spice },
    EglHeadless { egl_headless: EglHeadless },
    LookingGlass { looking_glass: LookingGlass },
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Gtk {}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Sdl {}

#[derive(Debug, Clone, Deserialize, Serialize, Getters)]
pub struct Vnc {
    #[serde(default)]
    listen: String,
    port: u16,
    #[serde(default)]
    gl_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    socket_path: Option<String>,
    #[serde(default)]
    password_auth: bool,
}

impl Vnc {
    pub fn new(listen: String, port: u16) -> Self {
        Self {
            listen,
            port,
            gl_enabled: false,
            socket_path: None,
            password_auth: false,
        }
    }

    pub fn with_gl_enabled(mut self, gl_enabled: bool) -> Self {
        self.gl_enabled = gl_enabled;
        self
    }

    pub fn with_socket_path(mut self, socket_path: Option<String>) -> Self {
        self.socket_path = socket_path;
        self
    }

    pub fn with_password_auth(mut self, password_auth: bool) -> Self {
        self.password_auth = password_auth;
        self
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Getters)]
pub struct Spice {
    #[serde(default)]
    listen: String,
    port: u16,
    #[serde(default)]
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

impl Spice {
    pub fn new(listen: String, port: u16, disable_ticketing: bool) -> Self {
        Self {
            listen,
            port,
            disable_ticketing,
            gl_enabled: false,
            tls_port: None,
            tls_ciphers: None,
            seamless_migration: false,
        }
    }

    pub fn with_gl_enabled(mut self, gl_enabled: bool) -> Self {
        self.gl_enabled = gl_enabled;
        self
    }

    pub fn with_tls_port(mut self, tls_port: Option<u16>) -> Self {
        self.tls_port = tls_port;
        self
    }

    pub fn with_tls_ciphers(mut self, tls_ciphers: Option<String>) -> Self {
        self.tls_ciphers = tls_ciphers;
        self
    }

    pub fn with_seamless_migration(mut self, seamless_migration: bool) -> Self {
        self.seamless_migration = seamless_migration;
        self
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct EglHeadless {}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LookingGlass {}

pub trait DisplayApi: fmt::Display {
    fn config(&self) -> &Display;
}

pub struct DisplayModelBuilder {}

impl DisplayModelBuilder {
    pub fn build(display: Display) -> Arc<dyn DisplayApi> {
        Arc::new(DisplayModel { display })
    }
}

struct DisplayModel {
    display: Display,
}

impl DisplayApi for DisplayModel {
    fn config(&self) -> &Display {
        &self.display
    }
}

impl fmt::Display for DisplayModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.display {
            Display::Gtk { .. } => write!(f, "GTK Display"),
            Display::Sdl { .. } => write!(f, "SDL Display"),
            Display::Vnc { vnc } => {
                if let Some(socket_path) = vnc.socket_path() {
                    write!(f, "VNC Display (unix:{socket_path})")
                } else {
                    write!(f, "VNC Display ({}:{})", vnc.listen, vnc.port)
                }
            }
            Display::Spice { spice } => {
                if let Some(tls_port) = spice.tls_port() {
                    write!(f, "SPICE Display ({}:{tls_port})", spice.listen)
                } else {
                    write!(f, "SPICE Display ({}:{})", spice.listen, spice.port)
                }
            }
            Display::EglHeadless { .. } => write!(f, "EGL Headless Display"),
            Display::LookingGlass { .. } => write!(f, "Looking Glass Display"),
        }
    }
}
