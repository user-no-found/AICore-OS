#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractVersion {
    pub contract_id: String,
    pub major: u16,
    pub minor: u16,
}

impl ContractVersion {
    pub fn new(contract_id: impl Into<String>, major: u16, minor: u16) -> Self {
        Self {
            contract_id: contract_id.into(),
            major,
            minor,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityRange {
    pub contract_id: String,
    pub min_major: u16,
    pub max_major: u16,
}

impl CompatibilityRange {
    pub fn accepts(&self, version: &ContractVersion) -> CompatibilityDecision {
        if self.contract_id == version.contract_id
            && version.major >= self.min_major
            && version.major <= self.max_major
        {
            CompatibilityDecision::Compatible
        } else {
            CompatibilityDecision::Incompatible
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompatibilityDecision {
    Compatible,
    Incompatible,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureFlag {
    pub name: String,
    pub enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::{CompatibilityDecision, CompatibilityRange, ContractVersion};

    #[test]
    fn contract_version_compatibility_accepts_supported_range() {
        let range = CompatibilityRange {
            contract_id: "kernel.route".to_string(),
            min_major: 1,
            max_major: 2,
        };

        assert_eq!(
            range.accepts(&ContractVersion::new("kernel.route", 1, 3)),
            CompatibilityDecision::Compatible
        );
    }

    #[test]
    fn contract_version_compatibility_rejects_unsupported_range() {
        let range = CompatibilityRange {
            contract_id: "kernel.route".to_string(),
            min_major: 1,
            max_major: 1,
        };

        assert_eq!(
            range.accepts(&ContractVersion::new("kernel.route", 2, 0)),
            CompatibilityDecision::Incompatible
        );
    }
}
