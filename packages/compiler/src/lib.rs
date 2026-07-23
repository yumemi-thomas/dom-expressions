mod compiler;
#[cfg(feature = "node")]
mod config;
#[cfg(feature = "node")]
mod directives;
mod dom;
mod error;
#[cfg(feature = "node")]
mod lazy;
#[cfg(feature = "node")]
mod node_adapter;
#[cfg(feature = "node")]
mod refresh;
mod semantic_trace;
mod shared;
mod ssr;
mod universal;

pub use compiler::{compile, CompileOptions, CompileOutput, Generate, Renderer, Wrapper};
pub use error::{CompileError, CompileErrorKind};
pub use semantic_trace::{
    CallbackDecision, ExecutionSite, ExecutionSiteKind, SemanticTrace, SourceSpan,
    TerminalDecision, ValueDecision,
};

/// Cargo package version of the compiler implementation producing semantic
/// traces. Consumers should pair this with the trace semantics revision.
pub const COMPILER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "node")]
pub use node_adapter::*;
