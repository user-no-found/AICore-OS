use crate::cli::CliArgs;
use crate::http::{ServerConfig, serve};

pub fn run() -> Result<(), String> {
    let args = CliArgs::parse(std::env::args().skip(1))?;
    if args.print_help {
        print_help();
        return Ok(());
    }
    if args.print_version {
        println!("aicore-web {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if let Some(root) = args.fpk_root {
        crate::fpk::write_package_source(&root)?;
        println!("fnOS 原生 FPK 包源目录已生成：{}", root.display());
        return Ok(());
    }
    serve(ServerConfig {
        host: args.host,
        port: args.port,
        once: args.once,
    })
}

fn print_help() {
    println!(
        "AICore Web 预留应用\n\n用法：\n  aicore-web [--host 0.0.0.0] [--port 8731]\n  aicore-web --once --host 127.0.0.1 --port 0\n  aicore-web --fpk-root <目录>\n\n说明：页面由 Vue3 提供，Rust 负责服务静态资源和未来运行时交互；当前不启动智能体运行时。fnOS 原生 FPK 使用 apps/aicore-web/packaging/fnos/scripts/package.sh 构建。"
    );
}
