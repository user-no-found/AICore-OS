use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliArgs {
    pub host: String,
    pub port: u16,
    pub once: bool,
    pub fpk_root: Option<PathBuf>,
    pub print_help: bool,
    pub print_version: bool,
}

impl CliArgs {
    pub fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut parsed = Self::default();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--host" => parsed.host = args.next().ok_or("--host 缺少值")?,
                "--port" => {
                    let value = args.next().ok_or("--port 缺少值")?;
                    parsed.port = value.parse().map_err(|_| format!("无效端口：{value}"))?;
                }
                "--once" => parsed.once = true,
                "--fpk-root" => {
                    parsed.fpk_root = Some(PathBuf::from(args.next().ok_or("--fpk-root 缺少值")?));
                }
                "--help" | "-h" => parsed.print_help = true,
                "--version" | "-V" => parsed.print_version = true,
                _ => return Err(format!("未知参数：{arg}")),
            }
        }
        Ok(parsed)
    }
}

impl Default for CliArgs {
    fn default() -> Self {
        Self {
            host: std::env::var("AICORE_WEB_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("AICORE_WEB_PORT")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(8731),
            once: false,
            fpk_root: None,
            print_help: false,
            print_version: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CliArgs;

    #[test]
    fn defaults_to_lan_bind_port() {
        let args = CliArgs::parse(Vec::<String>::new().into_iter()).unwrap();
        assert_eq!(args.host, "0.0.0.0");
        assert_eq!(args.port, 8731);
    }

    #[test]
    fn parses_host_port_once_and_fpk_root() {
        let args = CliArgs::parse(
            [
                "--host",
                "127.0.0.1",
                "--port",
                "9000",
                "--once",
                "--fpk-root",
                "target/fpk",
            ]
            .into_iter()
            .map(str::to_string),
        )
        .unwrap();
        assert_eq!(args.host, "127.0.0.1");
        assert_eq!(args.port, 9000);
        assert!(args.once);
        assert_eq!(args.fpk_root.unwrap().to_string_lossy(), "target/fpk");
    }
}
