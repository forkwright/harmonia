//! Library surface for the `archon` binary — exists so `tests/` integration crates can reach internals (e.g. `import::ImportAdapter`) that a bin-only crate cannot expose.

pub mod import;
pub mod mcp_bridge;
pub mod mcp_params;
