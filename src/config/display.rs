use std::any::Any;
#[allow(unused)]
pub use crate::config::display::gtk::Gtk;
#[allow(unused)]
pub use crate::config::display::looking_glass::LookingGlass;
#[allow(unused)]
pub use crate::config::display::no_display::NoDisplay;

use crate::config::QemuDevice;

mod gtk;
mod looking_glass;
mod no_display;
mod remote_viewer;

#[typetag::deserialize(tag = "type")]
pub trait Display:  'static + Any + QemuDevice {}
impl Default for Box<dyn Display> {
    fn default() -> Self {
        NoDisplay::boxed_default()
    }
}
