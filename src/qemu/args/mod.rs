//! QEMU argument management
//!
//! Structured representation of QEMU command-line arguments.

mod basic;
mod devices;
mod display;
mod guest_agent;
mod storage;
mod system;
mod tpm;
mod usb;

#[cfg(test)]
mod tests;
