pub mod capability;
pub mod component;
pub mod health;
pub mod instance;
pub mod lifecycle;
pub mod manifest;
pub mod permission;
pub mod registry;
pub mod versioning;

pub use capability::Capability;
pub use component::ComponentManifest;
pub use health::{HealthLevel, HealthStatus};
pub use instance::{InstanceKind, InstanceRecord};
pub use lifecycle::{InstallAction, LifecycleState};
pub use manifest::ManifestMetadata;
pub use permission::{CapabilityBoundary, PermissionBoundary};
pub use registry::{RegistrationRecord, RegistryKind};
pub use versioning::{Compatibility, VersionDescriptor};
