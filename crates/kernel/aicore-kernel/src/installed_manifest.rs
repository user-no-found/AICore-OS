use std::fs;
use std::path::Path;

use crate::{CapabilityDescriptor, CapabilityRegistry, ContractVersion};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentInvocationMode {
    InProcess,
    LocalProcess,
}

impl ComponentInvocationMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InProcess => "in_process",
            Self::LocalProcess => "local_process",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "local_process" => Self::LocalProcess,
            _ => Self::InProcess,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentTransport {
    StdioJsonl,
    UnixSocket,
    Unsupported,
}

impl ComponentTransport {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StdioJsonl => "stdio_jsonl",
            Self::UnixSocket => "unix_socket",
            Self::Unsupported => "unsupported",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "stdio_jsonl" => Self::StdioJsonl,
            "unix_socket" => Self::UnixSocket,
            _ => Self::Unsupported,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledComponentManifest {
    pub component_id: String,
    pub app_id: String,
    pub kind: String,
    pub entrypoint: String,
    pub invocation_mode: ComponentInvocationMode,
    pub transport: ComponentTransport,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub env_policy: Option<String>,
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
    pub invocation_mode: ComponentInvocationMode,
    pub transport: ComponentTransport,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub env_policy: Option<String>,
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
            "component_id = \"{}\"\napp_id = \"{}\"\nkind = \"{}\"\nentrypoint = \"{}\"\ninvocation_mode = \"{}\"\ntransport = \"{}\"\n",
            escape_toml(&self.component_id),
            escape_toml(&self.app_id),
            escape_toml(&self.kind),
            escape_toml(&self.entrypoint),
            self.invocation_mode.as_str(),
            self.transport.as_str(),
        );
        if !self.args.is_empty() {
            content.push_str(&format!(
                "args = [{}]\n",
                self.args
                    .iter()
                    .map(|arg| format!("\"{}\"", escape_toml(arg)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if let Some(working_dir) = &self.working_dir {
            content.push_str(&format!("working_dir = \"{}\"\n", escape_toml(working_dir)));
        }
        if let Some(env_policy) = &self.env_policy {
            content.push_str(&format!("env_policy = \"{}\"\n", escape_toml(env_policy)));
        }
        content.push_str(&format!(
            "contract_version = \"{}\"\n",
            escape_toml(&self.contract_version)
        ));
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
                        invocation_mode: manifest.invocation_mode.clone(),
                        transport: manifest.transport.clone(),
                        args: manifest.args.clone(),
                        working_dir: manifest.working_dir.clone(),
                        env_policy: manifest.env_policy.clone(),
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
    let mut invocation_mode = None;
    let mut transport = None;
    let mut args = Vec::new();
    let mut working_dir = None;
    let mut env_policy = None;
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
        let Some((key, raw_value)) = parse_key_value_raw(line) else {
            continue;
        };
        let value = parse_scalar_value(raw_value);
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
                "invocation_mode" => invocation_mode = Some(ComponentInvocationMode::parse(&value)),
                "transport" => transport = Some(ComponentTransport::parse(&value)),
                "args" => args = parse_array_value(raw_value),
                "working_dir" => working_dir = Some(value),
                "env_policy" => env_policy = Some(value),
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
        invocation_mode: invocation_mode.unwrap_or(ComponentInvocationMode::InProcess),
        transport: transport.unwrap_or(ComponentTransport::Unsupported),
        args,
        working_dir,
        env_policy,
        contract_version: required(contract_version, "contract_version")?,
        capabilities: capabilities
            .into_iter()
            .filter(|capability| !capability.id.is_empty() && !capability.operation.is_empty())
            .collect(),
    })
}

fn parse_key_value_raw(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once('=')?;
    Some((key.trim(), value.trim()))
}

fn parse_scalar_value(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}

fn parse_array_value(value: &str) -> Vec<String> {
    let value = value.trim();
    let Some(inner) = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    else {
        return vec![parse_scalar_value(value)];
    };
    let mut values = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escaped = false;
    for character in inner.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }
        match character {
            '\\' if in_quotes => escaped = true,
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                let item = current.trim();
                if !item.is_empty() {
                    values.push(parse_scalar_value(item));
                }
                current.clear();
            }
            value => current.push(value),
        }
    }
    let item = current.trim();
    if !item.is_empty() {
        values.push(parse_scalar_value(item));
    }
    values
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
    fn component_manifest_supports_invocation_mode_and_transport() {
        let root = temp_dir("manifest-process-metadata");
        fs::write(
            root.join("aicore-component-smoke.toml"),
            r#"
component_id = "aicore-component-smoke"
app_id = "aicore-cli"
kind = "app"
entrypoint = "/home/demo/.aicore/bin/aicore-cli"
invocation_mode = "local_process"
transport = "stdio_jsonl"
args = ["__component-smoke-stdio"]
working_dir = "/home/demo"
env_policy = "minimal"
contract_version = "kernel.app.v1"

[[capabilities]]
id = "component.process.smoke"
operation = "component.process.smoke"
visibility = "diagnostic"
"#,
        )
        .expect("write process manifest");

        let registry =
            InstalledManifestRegistry::load_from_dir(&root).expect("manifest registry should load");
        let manifest = &registry.manifests()[0];
        let candidate = registry
            .operation_candidates("component.process.smoke")
            .pop()
            .expect("process smoke route candidate");

        assert_eq!(manifest.invocation_mode.as_str(), "local_process");
        assert_eq!(manifest.transport.as_str(), "stdio_jsonl");
        assert_eq!(manifest.args, vec!["__component-smoke-stdio"]);
        assert_eq!(manifest.working_dir.as_deref(), Some("/home/demo"));
        assert_eq!(manifest.env_policy.as_deref(), Some("minimal"));
        assert_eq!(candidate.invocation_mode.as_str(), "local_process");
        assert_eq!(candidate.transport.as_str(), "stdio_jsonl");
        assert_eq!(candidate.args, vec!["__component-smoke-stdio"]);
    }

    #[test]
    fn component_manifest_defaults_to_in_process_for_existing_manifests() {
        let root = temp_dir("manifest-process-defaults");
        write_manifest(
            &root,
            "aicore-cli.toml",
            "aicore-cli",
            "kernel.app.v1",
            &[("memory.search", "memory.search")],
        );

        let registry =
            InstalledManifestRegistry::load_from_dir(&root).expect("manifest registry should load");
        let manifest = &registry.manifests()[0];
        let candidate = registry
            .operation_candidates("memory.search")
            .pop()
            .expect("memory.search route candidate");

        assert_eq!(manifest.invocation_mode.as_str(), "in_process");
        assert_eq!(manifest.transport.as_str(), "unsupported");
        assert!(manifest.args.is_empty());
        assert_eq!(candidate.invocation_mode.as_str(), "in_process");
        assert_eq!(candidate.transport.as_str(), "unsupported");
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
        assert_eq!(output.invocation_mode.as_str(), "in_process");
        assert_eq!(output.transport.as_str(), "unsupported");
    }

    #[test]
    fn route_decision_exposes_component_process_metadata() {
        let root = temp_dir("route-process-metadata");
        fs::write(
            root.join("aicore-component-smoke.toml"),
            r#"
component_id = "aicore-component-smoke"
app_id = "aicore-cli"
kind = "app"
entrypoint = "/home/demo/.aicore/bin/aicore-cli"
invocation_mode = "local_process"
transport = "stdio_jsonl"
args = ["__component-smoke-stdio"]
contract_version = "kernel.app.v1"

[[capabilities]]
id = "component.process.smoke"
operation = "component.process.smoke"
visibility = "diagnostic"
"#,
        )
        .expect("write process manifest");
        let registry = InstalledManifestRegistry::load_from_dir(&root).expect("registry");
        let runtime = KernelRouteRuntime::from_registry(registry);

        let output = runtime
            .route(KernelRouteRuntimeInput::new("component.process.smoke"))
            .expect("component process smoke should route");

        assert_eq!(output.component_id, "aicore-component-smoke");
        assert_eq!(output.invocation_mode.as_str(), "local_process");
        assert_eq!(output.transport.as_str(), "stdio_jsonl");
        assert_eq!(output.args, vec!["__component-smoke-stdio"]);
        assert!(!output.handler_executed);
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
