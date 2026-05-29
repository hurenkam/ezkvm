//! Render stage module scaffold.
//!
//! Owns deterministic argument rendering from an effective runtime model.

use crate::runtime_resolution::EffectiveRuntimeModel;

pub mod deterministic;

pub use deterministic::DeterministicRenderStage;

#[derive(Debug, Clone, Copy)]
pub struct RenderRequest<'a> {
    pub effective_runtime: &'a EffectiveRuntimeModel,
}

pub trait RenderStage {
    type Error;

    fn render(&self, request: RenderRequest<'_>) -> Result<Vec<String>, Self::Error>;
}
