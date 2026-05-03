use anyhow::{anyhow, Result};

use crate::token::is_valid_token;

#[derive(Debug)]
pub(crate) struct InstanceMetadata {
    pub(crate) instance_id: Option<String>,
    pub(crate) instance_kind: Option<String>,
}

pub(crate) fn parse_instance_metadata(contents: &str) -> Result<InstanceMetadata> {
    let mut instance_id = None;
    let mut instance_kind = None;

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"').to_string();
        match key.trim() {
            "instance_id" => {
                if !is_valid_token(&value) {
                    return Err(anyhow!("invalid instance id: {value}"));
                }
                instance_id = Some(value);
            }
            "instance_kind" => instance_kind = Some(value),
            _ => {}
        }
    }

    Ok(InstanceMetadata {
        instance_id,
        instance_kind,
    })
}
