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
pub struct InstalledRouteCandidate {
    pub component_id: String,
    pub app_id: String,
    pub entrypoint: String,
    pub contract_version: ContractVersion,
    pub capability_id: String,
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

    pub fn contract_version_descriptor(&self) -> ContractVersion {
        parse_contract_version(&self.contract_version)
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
            let contract = manifest.contract_version_descriptor();
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

    pub fn operation_candidates(&self, operation: &str) -> Vec<InstalledRouteCandidate> {
        self.manifests
            .iter()
            .flat_map(|manifest| {
                manifest
                    .capabilities
                    .iter()
                    .filter(move |capability| capability.operation == operation)
                    .map(move |capability| InstalledRouteCandidate {
                        component_id: manifest.component_id.clone(),
                        app_id: manifest.app_id.clone(),
                        entrypoint: manifest.entrypoint.clone(),
                        contract_version: manifest.contract_version_descriptor(),
                        capability_id: capability.id.clone(),
                        operation: capability.operation.clone(),
                        visibility: capability.visibility.clone(),
                    })
            })
            .collect()
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

    use crate::{ContractVersion, KernelRouteRuntime, KernelRouteRuntimeInput};

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

    #[test]
    fn installed_manifest_registry_routes_memory_search() {
        let root = temp_dir("route-memory-search");
        write_manifest(
            &root,
            "aicore-cli.toml",
            "aicore-cli",
            "kernel.app.v1",
            &[("memory.search", "memory.search")],
        );
        let registry = InstalledManifestRegistry::load_from_dir(&root).expect("registry");
        let runtime = KernelRouteRuntime::from_registry(registry);

        let output = runtime
            .route(KernelRouteRuntimeInput::new("memory.search"))
            .expect("memory.search should route");

        assert_eq!(output.component_id, "aicore-cli");
        assert_eq!(output.app_id, "aicore-cli");
        assert_eq!(output.capability_id, "memory.search");
        assert_eq!(output.decision.request.operation, "memory.search");
        assert_eq!(output.decision.target.app_id, "aicore-cli");
        assert_eq!(
            output.decision.target.contract_version.contract_id,
            "kernel.app"
        );
    }

    #[test]
    fn installed_manifest_registry_routes_provider_smoke() {
        let root = temp_dir("route-provider-smoke");
        write_manifest(
            &root,
            "aicore-cli.toml",
            "aicore-cli",
            "kernel.app.v1",
            &[("provider.smoke", "provider.smoke")],
        );
        let registry = InstalledManifestRegistry::load_from_dir(&root).expect("registry");
        let runtime = KernelRouteRuntime::from_registry(registry);

        let output = runtime
            .route(KernelRouteRuntimeInput::new("provider.smoke"))
            .expect("provider.smoke should route");

        assert_eq!(output.component_id, "aicore-cli");
        assert_eq!(output.capability_id, "provider.smoke");
        assert!(!output.handler_executed);
    }

    #[test]
    fn installed_manifest_registry_rejects_missing_operation() {
        let root = temp_dir("route-missing-operation");
        write_manifest(
            &root,
            "aicore-cli.toml",
            "aicore-cli",
            "kernel.app.v1",
            &[("memory.search", "memory.search")],
        );
        let registry = InstalledManifestRegistry::load_from_dir(&root).expect("registry");
        let runtime = KernelRouteRuntime::from_registry(registry);

        let error = runtime
            .route(KernelRouteRuntimeInput::new("unknown.operation"))
            .expect_err("unknown operation should fail");

        assert!(error.to_string().contains("missing capability"));
        assert!(error.to_string().contains("unknown.operation"));
    }

    #[test]
    fn installed_manifest_registry_missing_manifest_dir_returns_no_route() {
        let root = temp_dir("route-missing-dir").join("missing");
        let registry = InstalledManifestRegistry::load_from_dir(&root).expect("registry");
        let runtime = KernelRouteRuntime::from_registry(registry);

        let error = runtime
            .route(KernelRouteRuntimeInput::new("memory.search"))
            .expect_err("missing manifest dir should have no route");

        assert!(error.to_string().contains("missing capability"));
    }

    #[test]
    fn route_decision_rejects_contract_version_mismatch() {
        let root = temp_dir("route-contract-mismatch");
        write_manifest(
            &root,
            "aicore-cli.toml",
            "aicore-cli",
            "kernel.app.v2",
            &[("memory.search", "memory.search")],
        );
        let registry = InstalledManifestRegistry::load_from_dir(&root).expect("registry");
        let runtime = KernelRouteRuntime::from_registry(registry);

        let error = runtime
            .route(KernelRouteRuntimeInput::new("memory.search"))
            .expect_err("contract mismatch should fail");

        assert!(error.to_string().contains("contract version mismatch"));
    }

    #[test]
    fn route_decision_rejects_requested_contract_version_mismatch() {
        let root = temp_dir("route-requested-contract-mismatch");
        write_manifest(
            &root,
            "aicore-cli.toml",
            "aicore-cli",
            "kernel.app.v1",
            &[("memory.search", "memory.search")],
        );
        let registry = InstalledManifestRegistry::load_from_dir(&root).expect("registry");
        let runtime = KernelRouteRuntime::from_registry(registry);

        let error = runtime
            .route(
                KernelRouteRuntimeInput::new("memory.search")
                    .with_requested_contract(ContractVersion::new("kernel.app", 2, 0)),
            )
            .expect_err("requested contract mismatch should fail");

        assert!(error.to_string().contains("contract version mismatch"));
    }

    #[test]
    fn route_decision_rejects_ambiguous_duplicate_capability() {
        let root = temp_dir("route-duplicate-capability");
        write_manifest(
            &root,
            "aicore-cli.toml",
            "aicore-cli",
            "kernel.app.v1",
            &[("memory.search", "memory.search")],
        );
        write_manifest(
            &root,
            "aicore-memory.toml",
            "aicore-memory",
            "kernel.app.v1",
            &[("memory.search", "memory.search")],
        );
        let registry = InstalledManifestRegistry::load_from_dir(&root).expect("registry");
        let runtime = KernelRouteRuntime::from_registry(registry);

        let error = runtime
            .route(KernelRouteRuntimeInput::new("memory.search"))
            .expect_err("duplicate operation should be ambiguous");

        assert!(error.to_string().contains("ambiguous route"));
        assert!(error.to_string().contains("aicore-cli"));
        assert!(error.to_string().contains("aicore-memory"));
    }

    fn write_manifest(
        root: &PathBuf,
        file_name: &str,
        app_id: &str,
        contract_version: &str,
        capabilities: &[(&str, &str)],
    ) {
        let mut content = format!(
            "component_id = \"{app_id}\"\napp_id = \"{app_id}\"\nkind = \"app\"\nentrypoint = \"/home/demo/.aicore/bin/{app_id}\"\ncontract_version = \"{contract_version}\"\n"
        );
        for (id, operation) in capabilities {
            content.push_str(&format!(
                "\n[[capabilities]]\nid = \"{id}\"\noperation = \"{operation}\"\nvisibility = \"user\"\n"
            ));
        }
        fs::write(root.join(file_name), content).expect("write manifest");
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
