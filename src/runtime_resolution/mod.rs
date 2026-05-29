//! Runtime resolution stage module scaffold.
//!
//! Owns runtime-host binding and capability resolution for vm_spec input.

#[derive(Debug, Default)]
pub struct RuntimeResolutionStage;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EffectiveRuntimeModel {
    pub qemu_args: Vec<String>,
}
