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
}

#[derive(Debug, Clone, Deserialize, Serialize, Getters)]
pub struct Spice {
    #[serde(default)]
    listen: String,
    port: u16,
    #[serde(default)]
    disable_ticketing: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LookingGlass {}

pub trait DisplayApi: fmt::Display {
    fn config(&self) -> &Display;
    fn qemu_args(&self) -> Vec<String>;
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

    fn qemu_args(&self) -> Vec<String> {
        match &self.display {
            Display::Gtk { .. } => vec!["-display".to_string(), "gtk".to_string()],
            Display::Sdl { .. } => vec!["-display".to_string(), "sdl".to_string()],
            Display::Vnc { vnc } => {
                let listen = if vnc.listen.is_empty() {
                    "0.0.0.0"
                } else {
                    &vnc.listen
                };
                vec!["-vnc".to_string(), format!("{}:{}", listen, vnc.port)]
            }
            Display::Spice { spice } => {
                let listen = if spice.listen.is_empty() {
                    "0.0.0.0"
                } else {
                    &spice.listen
                };
                let mut spec = format!("port={},addr={}", spice.port, listen);
                if spice.disable_ticketing {
                    spec.push_str(",disable-ticketing=on");
                }
                vec!["-spice".to_string(), spec]
            }
            Display::LookingGlass { .. } => vec![
                "-display".to_string(),
                "none".to_string(),
                "-vnc".to_string(),
                "none".to_string(),
            ],
        }
    }
}

impl fmt::Display for DisplayModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.display {
            Display::Gtk { .. } => write!(f, "GTK Display"),
            Display::Sdl { .. } => write!(f, "SDL Display"),
            Display::Vnc { vnc } => write!(f, "VNC Display ({}:{})", vnc.listen, vnc.port),
            Display::Spice { spice } => {
                write!(f, "SPICE Display ({}:{})", spice.listen, spice.port)
            }
            Display::LookingGlass { .. } => write!(f, "Looking Glass Display"),
        }
    }
}
