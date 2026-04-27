mod cargo_diagnostics;
mod cargo_runner;
mod layers;
mod runner;
mod workflow_output;

use std::env;
use std::process::ExitCode;

use layers::Workflow;

fn main() -> ExitCode {
    let Some(arg) = env::args().nth(1) else {
        eprintln!(
            "缺少 workflow 参数。可用值：foundation、kernel、core、app-aicore、app-cli、app-tui"
        );
        return ExitCode::from(1);
    };

    let workflow = match Workflow::parse(&arg) {
        Some(value) => value,
        None => {
            eprintln!("未知 workflow: {arg}");
            return ExitCode::from(1);
        }
    };

    match runner::run(workflow) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}
