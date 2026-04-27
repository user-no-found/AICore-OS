#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaRef {
    pub schema_id: String,
    pub version: String,
}

impl SchemaRef {
    pub fn new(schema_id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            schema_id: schema_id.into(),
            version: version.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialRequirement {
    pub auth_ref: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxRequirement {
    pub boundary: String,
    pub network: bool,
    pub filesystem: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityOperation {
    pub operation: String,
    pub input_schema: Option<SchemaRef>,
    pub output_schema: Option<SchemaRef>,
}

impl CapabilityOperation {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            input_schema: None,
            output_schema: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDescriptor {
    pub capability_id: String,
    pub operations: Vec<CapabilityOperation>,
    pub schema_refs: Vec<SchemaRef>,
    pub credential_requirement: Option<CredentialRequirement>,
    pub sandbox_requirement: Option<SandboxRequirement>,
}

impl CapabilityDescriptor {
    pub fn new(capability_id: impl Into<String>) -> Self {
        Self {
            capability_id: capability_id.into(),
            operations: Vec::new(),
            schema_refs: Vec::new(),
            credential_requirement: None,
            sandbox_requirement: None,
        }
    }

    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operations.push(CapabilityOperation::new(operation));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{CapabilityDescriptor, SchemaRef};

    #[test]
    fn capability_descriptor_declares_operations_and_schema_refs() {
        let mut descriptor = CapabilityDescriptor::new("memory.search").with_operation("search");
        descriptor
            .schema_refs
            .push(SchemaRef::new("schema.memory.search", "1.0"));

        assert_eq!(descriptor.operations[0].operation, "search");
        assert_eq!(descriptor.schema_refs[0].schema_id, "schema.memory.search");
    }
}
