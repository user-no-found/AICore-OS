pub fn blocks_secret(content: &str) -> bool {
    let lowered = content.to_ascii_lowercase();
    lowered.contains("sk-")
        || lowered.contains("secret://")
        || lowered.contains("api_key=")
        || lowered.contains("api_key =")
        || lowered.contains("\"api_key\":")
        || lowered.contains("'api_key':")
}
