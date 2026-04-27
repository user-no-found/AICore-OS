use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::{ProviderEngineEvent, ProviderEngineRequest, ProviderError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderEngineManager {
    pub python_executable: String,
    pub engine_package_root: PathBuf,
}

impl ProviderEngineManager {
    pub fn new(
        python_executable: impl Into<String>,
        engine_package_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            python_executable: python_executable.into(),
            engine_package_root: engine_package_root.into(),
        }
    }

    pub fn default_for_crate() -> Self {
        Self::new(
            "python3",
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("python"),
        )
    }

    pub fn invoke_fake(
        &self,
        request: ProviderEngineRequest,
    ) -> Result<Vec<ProviderEngineEvent>, ProviderError> {
        self.invoke_worker("fake", request)
    }

    pub fn invoke_python_engine(
        &self,
        engine_id: &str,
        request: ProviderEngineRequest,
    ) -> Result<Vec<ProviderEngineEvent>, ProviderError> {
        match engine_id {
            "python.fake" => self.invoke_worker("fake", request),
            "python.openai" => self.invoke_worker("openai", request),
            "python.anthropic" => self.invoke_worker("anthropic", request),
            "python.codex_bridge" => Err(ProviderError::Invoke(
                "Provider engine 不可用（codex_bridge_unavailable）".to_string(),
            )),
            other => Err(ProviderError::Invoke(format!(
                "Provider engine 不可用（{other}）"
            ))),
        }
    }

    fn invoke_worker(
        &self,
        worker_engine: &str,
        request: ProviderEngineRequest,
    ) -> Result<Vec<ProviderEngineEvent>, ProviderError> {
        let request_json = serde_json::to_string(&request)
            .map_err(|error| ProviderError::Invoke(error.to_string()))?;
        let mut child = Command::new(&self.python_executable)
            .arg("-m")
            .arg("aicore_provider_engine.worker")
            .arg("--engine")
            .arg(worker_engine)
            .env("PYTHONPATH", &self.engine_package_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                ProviderError::Invoke(format!("Provider engine 启动失败（{error}）"))
            })?;

        child
            .stdin
            .as_mut()
            .ok_or_else(|| ProviderError::Invoke("Provider engine stdin 不可用".to_string()))?
            .write_all(format!("{request_json}\n").as_bytes())
            .map_err(|error| {
                ProviderError::Invoke(format!("Provider engine 写入失败（{error}）"))
            })?;
        drop(child.stdin.take());

        let output = child.wait_with_output().map_err(|error| {
            ProviderError::Invoke(format!("Provider engine 等待失败（{error}）"))
        })?;

        if !output.status.success() {
            return Err(ProviderError::Invoke(
                "Provider engine 执行失败（worker_exit）".to_string(),
            ));
        }

        String::from_utf8(output.stdout)
            .map_err(|error| ProviderError::Invoke(format!("Provider engine 输出无效（{error}）")))?
            .lines()
            .map(|line| {
                serde_json::from_str(line).map_err(|error| {
                    ProviderError::Invoke(format!("Provider engine 事件无效（{error}）"))
                })
            })
            .collect()
    }
}
