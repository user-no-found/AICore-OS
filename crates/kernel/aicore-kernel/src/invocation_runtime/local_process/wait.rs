use std::io::Read;
use std::process::Child;
use std::time::{Duration, Instant};

use crate::{KernelInvocationEnvelope, TimeoutPolicy};

use super::{ComponentProcessFailure, ComponentProcessOutput};

const DEFAULT_COMPONENT_PROCESS_TIMEOUT_MS: u64 = 30_000;
const PROCESS_POLL_MS: u64 = 10;

pub(super) fn timeout_duration(envelope: &KernelInvocationEnvelope) -> Option<Duration> {
    match envelope.policy.timeout {
        TimeoutPolicy::Inherit => Some(Duration::from_millis(DEFAULT_COMPONENT_PROCESS_TIMEOUT_MS)),
        TimeoutPolicy::Millis(value) => Some(Duration::from_millis(value)),
        TimeoutPolicy::None => None,
    }
}

pub(super) fn wait_with_timeout(
    mut child: Child,
    timeout: Option<Duration>,
) -> Result<ComponentProcessOutput, ComponentProcessFailure> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = read_pipe(child.stdout.take(), "stdout")?;
                let stderr = read_pipe(child.stderr.take(), "stderr")?;
                return Ok(ComponentProcessOutput {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if timeout.is_some_and(|timeout| started.elapsed() >= timeout) {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(ComponentProcessFailure {
                        stage: "process_timeout".to_string(),
                        reason: "component process timed out and was terminated".to_string(),
                        spawned_process: true,
                        exit_code: None,
                    });
                }
                std::thread::sleep(Duration::from_millis(PROCESS_POLL_MS));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ComponentProcessFailure {
                    stage: "process_stdout_failed".to_string(),
                    reason: super::super::protocol::sanitize_process_diagnostic(&format!(
                        "component process status read failed: {error}"
                    )),
                    spawned_process: true,
                    exit_code: None,
                });
            }
        }
    }
}

fn read_pipe(pipe: Option<impl Read>, name: &str) -> Result<Vec<u8>, ComponentProcessFailure> {
    let Some(mut pipe) = pipe else {
        return Ok(Vec::new());
    };
    let mut buffer = Vec::new();
    pipe.read_to_end(&mut buffer)
        .map_err(|error| ComponentProcessFailure {
            stage: "process_stdout_failed".to_string(),
            reason: super::super::protocol::sanitize_process_diagnostic(&format!(
                "component process {name} read failed: {error}"
            )),
            spawned_process: true,
            exit_code: None,
        })?;
    Ok(buffer)
}
