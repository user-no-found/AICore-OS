use crate::commands::memory::write::{
    build_memory_accept_write_report, build_memory_reject_write_report,
    build_memory_remember_write_report, memory_write_failure_fields,
};

use super::component_stdio::{payload_string, run_component_write_report_stdio_with_request};

pub(crate) fn run_component_memory_remember_stdio() -> i32 {
    run_component_write_report_stdio_with_request(
        "memory.remember",
        "memory remember component stdin 读取失败",
        |request, _error| {
            let content = payload_string(request, "content", "");
            memory_write_failure_fields("memory.remember", None, Some(content.chars().count()))
        },
        |request| {
            let content = payload_string(request, "content", "");
            build_memory_remember_write_report(&content)
        },
    )
}

pub(crate) fn run_component_memory_accept_stdio() -> i32 {
    run_component_write_report_stdio_with_request(
        "memory.accept",
        "memory accept component stdin 读取失败",
        |request, _error| {
            let proposal_id = payload_string(request, "proposal_id", "");
            memory_write_failure_fields("memory.accept", Some(proposal_id), None)
        },
        |request| {
            let proposal_id = payload_string(request, "proposal_id", "");
            build_memory_accept_write_report(&proposal_id)
        },
    )
}

pub(crate) fn run_component_memory_reject_stdio() -> i32 {
    run_component_write_report_stdio_with_request(
        "memory.reject",
        "memory reject component stdin 读取失败",
        |request, _error| {
            let proposal_id = payload_string(request, "proposal_id", "");
            memory_write_failure_fields("memory.reject", Some(proposal_id), None)
        },
        |request| {
            let proposal_id = payload_string(request, "proposal_id", "");
            build_memory_reject_write_report(&proposal_id)
        },
    )
}
