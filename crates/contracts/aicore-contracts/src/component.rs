use crate::{Capability, Compatibility, ManifestMetadata, PermissionBoundary, VersionDescriptor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentManifest {
    pub id: String,
    pub kind: String,
    pub manifest: ManifestMetadata,
    pub display_name_zh: String,
    pub description_zh: String,
    pub version: VersionDescriptor,
    pub compatibility: Compatibility,
    pub permission_boundary: PermissionBoundary,
    pub capabilities: Vec<Capability>,
}
