#[allow(unused)]
pub use gtk::Gtk;
#[allow(unused)]
pub use looking_glass::LookingGlass;
#[allow(unused)]
pub use no_display::NoDisplay;
use std::any::Any;

use super::QemuDevice;

mod gtk;
mod looking_glass;
mod no_display;
mod remote_viewer;

#[typetag::deserialize(tag = "type")]
pub trait Display: 'static + Any + QemuDevice {}
impl Default for Box<dyn Display> {
    fn default() -> Self {
        NoDisplay::boxed_default()
    }
}
