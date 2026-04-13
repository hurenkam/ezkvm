//! QEMU argument management
//!
//! Structured representation of QEMU command-line arguments.

mod basic;
mod devices;
mod display;
mod storage;
mod system;

#[cfg(test)]
mod tests;
