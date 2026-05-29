//! Deterministic render adapter.
//!
//! Owns transformation from effective runtime model to QEMU argument list.

use std::convert::Infallible;

use super::{RenderRequest, RenderStage};

#[derive(Debug, Default)]
pub struct DeterministicRenderStage;

impl RenderStage for DeterministicRenderStage {
    type Error = Infallible;

    fn render(&self, request: RenderRequest<'_>) -> Result<Vec<String>, Self::Error> {
        Ok(request.effective_runtime.qemu_args.clone())
    }
}
