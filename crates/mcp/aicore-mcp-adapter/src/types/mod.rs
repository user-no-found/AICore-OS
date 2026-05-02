mod enums;
mod ids;
mod mapping;
mod result;
mod server;
mod tool;

pub use enums::*;
pub use ids::*;
pub use mapping::*;
pub use result::*;
pub use server::*;
pub use tool::*;

pub fn exported_contract_symbols() -> &'static [&'static str] {
    &[
        "McpServerDescriptor",
        "McpToolCandidate",
        "McpToolToAicoreToolMapping",
        "McpPermissionHook",
        "McpResultSummaryBoundary",
        "McpDiscoveryReport",
    ]
}
