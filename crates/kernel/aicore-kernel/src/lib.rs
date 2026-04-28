pub mod app;
pub mod capability;
pub mod context;
pub mod error;
pub mod event;
pub mod installed_manifest;
pub mod invocation;
pub mod invocation_runtime;
pub mod registry;
pub mod route;
pub mod route_runtime;
pub mod runtime;
pub mod scheduler;
pub mod versioning;

pub use app::*;
pub use capability::*;
pub use context::*;
pub use error::*;
pub use event::*;
pub use installed_manifest::*;
pub use invocation::*;
pub use invocation_runtime::*;
pub use registry::*;
pub use route::*;
pub use route_runtime::*;
pub use runtime::*;
pub use scheduler::*;
pub use versioning::*;

pub fn default_runtime() -> InstanceRuntime {
    InstanceRuntime::new("global-main", "main")
}

#[cfg(test)]
mod tests {
    use super::{AppManifest, CapabilityDescriptor, KernelRouteRequest};

    #[test]
    fn kernel_crate_exports_core_types() {
        let _manifest = AppManifest::new("app.cli", "cli");
        let _capability = CapabilityDescriptor::new("provider.chat");
        let _route = KernelRouteRequest::new("global-main", "provider.chat", "complete");
    }
}
