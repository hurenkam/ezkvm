# Runtime Resolution And Render Stage

Back to index: [Current Implementation Architecture](./current-implementation-architecture.md)

## Scope

This note covers stage boundaries in:

- `src/runtime_resolution/mod.rs`
- `src/render_stage/mod.rs`
- `src/render_stage/deterministic.rs`

`RuntimeResolutionStage` and `EffectiveRuntimeModel` currently form a scaffold boundary. `DeterministicRenderStage` is the active renderer implementation.

## Stage Structure

```plantuml
@startuml
skinparam classAttributeIconSize 0

class RuntimeResolutionStage << (S,#98FB98) >>

class EffectiveRuntimeModel << (S,#98FB98) >> {
  +qemu_args: Vec<String>
}

class RenderRequest << (S,#98FB98) >> {
  +effective_runtime: &EffectiveRuntimeModel
}

interface RenderStage << (T,#FFB347) >> {
  +render(request: RenderRequest) -> Result<Vec<String>, Error>
}

class DeterministicRenderStage << (S,#98FB98) >> {
  +render(request) -> Result<Vec<String>, Infallible>
}

RuntimeResolutionStage --> EffectiveRuntimeModel
RenderRequest --> EffectiveRuntimeModel
RenderStage <|.. DeterministicRenderStage
DeterministicRenderStage ..> EffectiveRuntimeModel
@enduml
```

## Current Behavior

- Runtime resolution methods are not yet implemented beyond publishing the effective model shape.
- Deterministic rendering is a pass-through clone of `EffectiveRuntimeModel.qemu_args`.
- The render trait boundary is synchronous and object-safe.

## Render Flow

```plantuml
@startuml
actor Caller
participant "DeterministicRenderStage" as Renderer
participant "EffectiveRuntimeModel" as Runtime

Caller -> Renderer : render(RenderRequest)
Renderer -> Runtime : read qemu_args
Runtime --> Renderer : Vec<String>
Renderer --> Caller : cloned Vec<String>
@enduml
```

## Related Docs

- [Model Separation Pipeline Contract](../model-separation-pipeline.md)
- [VM Spec, Parsing, And Validation](./vm-spec-parsing-validation.md)
