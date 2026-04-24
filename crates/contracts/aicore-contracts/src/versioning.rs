#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionDescriptor {
    pub current: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compatibility {
    pub min_runtime_version: String,
}
