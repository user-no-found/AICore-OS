use std::fs;
use std::path::Path;

use crate::{CapabilityDescriptor, CapabilityRegistry, ContractVersion};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledComponentManifest {
    pub component_id: String,
    pub app_id: String,
    pub kind: String,
    pub entrypoint: String,
    pub contract_version: String,
    pub capabilities: Vec<InstalledCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledCapability {
    pub id: String,
    pub operation: String,
    pub visibility: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledManifestRegistry {
    manifests: Vec<InstalledComponentManifest>,
}

impl InstalledComponentManifest {
    pub fn to_toml(&self) -> String {
        let mut content = format!(
            "component_id = \"{}\"\napp_id = \"{}\"\nkind = \"{}\"\nentrypoint = \"{}\"\ncontract_version = \"{}\"\n",
            escape_toml(&self.component_id),
            escape_toml(&self.app_id),
            escape_toml(&self.kind),
            escape_toml(&self.entrypoint),
            escape_toml(&self.contract_version)
        );
        for capability in &self.capabilities {
            content.push_str(&format!(
                "\n[[capabilities]]\nid = \"{}\"\noperation = \"{}\"\nvisibility = \"{}\"\n",
                escape_toml(&capability.id),
                escape_toml(&capability.operation),
                escape_toml(&capability.visibility)
            ));
        }
        content
    }
}

impl InstalledManifestRegistry {
    pub fn load_from_dir(path: &Path) -> Result<Self, String> {
        let mut manifests = Vec::new();
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self { manifests });
            }
            Err(error) => {
                return Err(format!(
                    "读取 manifest 目录 {} 失败: {error}",
                    path.display()
                ));
            }
        };
        let mut files = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("toml"))
            .collect::<Vec<_>>();
        files.sort();

        for file in files {
            let content = fs::read_to_string(&file)
                .map_err(|error| format!("读取 manifest {} 失败: {error}", file.display()))?;
            manifests.push(parse_manifest(&content)?);
        }

        Ok(Self { manifests })
    }

    pub fn from_manifests(manifests: Vec<InstalledComponentManifest>) -> Self {
        Self { manifests }
    }

    pub fn manifests(&self) -> &[InstalledComponentManifest] {
        &self.manifests
    }

    pub fn manifest_count(&self) -> usize {
        self.manifests.len()
    }

    pub fn capability_count(&self) -> usize {
        self.manifests
            .iter()
            .map(|manifest| manifest.capabilities.len())
            .sum()
    }

    pub fn to_capability_registry(&self) -> CapabilityRegistry {
        let mut registry = CapabilityRegistry::new();
        for manifest in &self.manifests {
            let contract = parse_contract_version(&manifest.contract_version);
            for capability in &manifest.capabilities {
                registry.register(
                    manifest.app_id.clone(),
                    CapabilityDescriptor::new(capability.id.clone())
                        .with_operation(capability.operation.clone()),
                    contract.clone(),
                );
            }
        }
        registry
    }
}

fn parse_manifest(content: &str) -> Result<InstalledComponentManifest, String> {
    let mut component_id = None;
    let mut app_id = None;
    let mut kind = None;
    let mut entrypoint = None;
    let mut contract_version = None;
    let mut capabilities = Vec::new();
    let mut current_capability: Option<InstalledCapability> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[[capabilities]]" {
            if let Some(capability) = current_capability.take() {
                capabilities.push(capability);
            }
            current_capability = Some(InstalledCapability {
                id: String::new(),
                operation: String::new(),
                visibility: String::new(),
            });
            continue;
        }
        let Some((key, value)) = parse_key_value(line) else {
            continue;
        };
        if let Some(capability) = current_capability.as_mut() {
            match key {
                "id" => capability.id = value,
                "operation" => capability.operation = value,
                "visibility" => capability.visibility = value,
                _ => {}
            }
        } else {
            match key {
                "component_id" => component_id = Some(value),
                "app_id" => app_id = Some(value),
                "kind" => kind = Some(value),
                "entrypoint" => entrypoint = Some(value),
                "contract_version" => contract_version = Some(value),
                _ => {}
            }
        }
    }
    if let Some(capability) = current_capability.take() {
        capabilities.push(capability);
    }

    Ok(InstalledComponentManifest {
        component_id: required(component_id, "component_id")?,
        app_id: required(app_id, "app_id")?,
        kind: required(kind, "kind")?,
        entrypoint: required(entrypoint, "entrypoint")?,
        contract_version: required(contract_version, "contract_version")?,
        capabilities: capabilities
            .into_iter()
            .filter(|capability| !capability.id.is_empty() && !capability.operation.is_empty())
            .collect(),
    })
}

fn parse_key_value(line: &str) -> Option<(&str, String)> {
    let (key, value) = line.split_once('=')?;
    Some((key.trim(), value.trim().trim_matches('"').to_string()))
}

fn required(value: Option<String>, key: &str) -> Result<String, String> {
    value.ok_or_else(|| format!("manifest 缺少字段: {key}"))
}

fn parse_contract_version(value: &str) -> ContractVersion {
    if let Some((contract_id, major)) = value.rsplit_once(".v") {
        return ContractVersion::new(contract_id, major.parse::<u16>().unwrap_or(1), 0);
    }
    ContractVersion::new(value, 1, 0)
}

fn escape_toml(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::InstalledManifestRegistry;

    #[test]
    fn installed_manifest_loader_reads_component_manifest() {
        let root = temp_dir("manifest-loader");
        fs::write(
            root.join("aicore-cli.toml"),
            r#"
component_id = "aicore-cli"
app_id = "aicore-cli"
kind = "app"
entrypoint = "/home/demo/.aicore/bin/aicore-cli"
contract_version = "kernel.app.v1"

[[capabilities]]
id = "memory.status"
operation = "memory.status"
visibility = "user"

[[capabilities]]
id = "memory.search"
operation = "memory.search"
visibility = "user"
"#,
        )
        .expect("write manifest");

        let registry =
            InstalledManifestRegistry::load_from_dir(&root).expect("manifest registry should load");

        assert_eq!(registry.manifest_count(), 1);
        assert_eq!(registry.capability_count(), 2);
        assert_eq!(registry.manifests()[0].component_id, "aicore-cli");
        assert_eq!(
            registry.manifests()[0].capabilities[0].operation,
            "memory.status"
        );
    }

    #[test]
    fn installed_manifest_registry_builds_capability_registry() {
        let root = temp_dir("capability-registry");
        fs::write(
            root.join("aicore.toml"),
            r#"
component_id = "aicore"
app_id = "aicore"
kind = "app"
entrypoint = "/home/demo/.aicore/bin/aicore"
contract_version = "kernel.app.v1"

[[capabilities]]
id = "runtime.status"
operation = "runtime.status"
visibility = "user"
"#,
        )
        .expect("write manifest");

        let registry =
            InstalledManifestRegistry::load_from_dir(&root).expect("manifest registry should load");
        let capability_registry = registry.to_capability_registry();
        let entry = capability_registry
            .find("runtime.status", "runtime.status")
            .expect("runtime.status should be registered");

        assert_eq!(entry.app_id, "aicore");
        assert_eq!(entry.contract_version.contract_id, "kernel.app");
        assert_eq!(entry.contract_version.major, 1);
    }

    fn temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "aicore-kernel-{name}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }
}
