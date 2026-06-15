use std::fmt::Display;

pub struct Cdrom {}
impl Display for Cdrom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cdrom")
    }
}

pub struct Hdd {}
impl Display for Hdd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hdd")
    }
}

pub struct Ssd {}
impl Display for Ssd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ssd")
    }
}
