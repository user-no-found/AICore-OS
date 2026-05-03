use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

const INDEX_HTML: &str = include_str!("../web/dist/index.html");
const APP_JS: &str = include_str!("../web/dist/assets/app.js");
const APP_CSS: &str = include_str!("../web/dist/assets/app.css");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub once: bool,
}

pub fn serve(config: ServerConfig) -> Result<(), String> {
    let listener = TcpListener::bind((config.host.as_str(), config.port))
        .map_err(|error| format!("监听 {}:{} 失败：{error}", config.host, config.port))?;
    let addr = listener
        .local_addr()
        .map_err(|error| format!("读取监听地址失败：{error}"))?;
    println!("AICore Web 已启动：http://{addr}");
    println!("当前为 Vue3 预留界面；Rust 后端只提供静态资源和状态接口。");
    for stream in listener.incoming() {
        let stream = stream.map_err(|error| format!("接收连接失败：{error}"))?;
        handle_stream(stream)?;
        if config.once {
            break;
        }
    }
    Ok(())
}

fn handle_stream(mut stream: TcpStream) -> Result<(), String> {
    let mut request_line = String::new();
    {
        let mut reader = BufReader::new(&mut stream);
        reader
            .read_line(&mut request_line)
            .map_err(|error| format!("读取请求失败：{error}"))?;
    }
    let path = request_path(&request_line);
    let response = match path {
        "/" | "/index.html" => response("200 OK", "text/html; charset=utf-8", INDEX_HTML),
        "/assets/app.js" => response("200 OK", "text/javascript; charset=utf-8", APP_JS),
        "/assets/app.css" => response("200 OK", "text/css; charset=utf-8", APP_CSS),
        "/health" => response(
            "200 OK",
            "application/json; charset=utf-8",
            &crate::status::health_json(),
        ),
        "/api/status" => response(
            "200 OK",
            "application/json; charset=utf-8",
            &crate::status::status_json(),
        ),
        _ => response("404 Not Found", "text/plain; charset=utf-8", "未找到页面"),
    };
    stream
        .write_all(response.as_bytes())
        .map_err(|error| format!("写入响应失败：{error}"))
}

fn request_path(line: &str) -> &str {
    let mut parts = line.split_whitespace();
    let _method = parts.next();
    parts.next().unwrap_or("/")
}

fn response(status: &str, content_type: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_request_path() {
        assert_eq!(
            super::request_path("GET /api/status HTTP/1.1"),
            "/api/status"
        );
        assert_eq!(super::request_path(""), "/");
    }

    #[test]
    fn serves_vue_entry_asset_names() {
        assert!(super::INDEX_HTML.contains("/assets/app.js"));
        assert!(super::INDEX_HTML.contains("/assets/app.css"));
        assert!(super::APP_JS.contains("createApp"));
    }
}
