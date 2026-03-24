// Renderer mode: headless audio endpoint receiving streams via QUIC.

pub mod config;
pub mod error;
pub mod pipeline;
pub mod protocol;
pub mod runner;
pub mod server;
pub mod status;
pub mod tls;

pub use error::RenderError;
pub use runner::run_render;
pub use server::RendererRegistry;
