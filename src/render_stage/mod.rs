//! Render stage module scaffold.
//!
//! Owns deterministic argument rendering from an effective runtime model.

use std::convert::Infallible;

use crate::runtime_resolution::EffectiveRuntimeModel;

#[derive(Debug, Clone, Copy)]
pub struct RenderRequest<'a> {
    pub effective_runtime: &'a EffectiveRuntimeModel,
}

pub trait RenderStage {
    type Error;

    fn render(&self, request: RenderRequest<'_>) -> Result<Vec<String>, Self::Error>;
}

#[derive(Debug, Default)]
pub struct DeterministicRenderStage;

impl RenderStage for DeterministicRenderStage {
    type Error = Infallible;

    fn render(&self, request: RenderRequest<'_>) -> Result<Vec<String>, Self::Error> {
        Ok(request.effective_runtime.qemu_args.clone())
    }
}
