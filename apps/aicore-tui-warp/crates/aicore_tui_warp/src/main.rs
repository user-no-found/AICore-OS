mod root_view;

use std::borrow::Cow;

use anyhow::{anyhow, Context, Result};
use rust_embed::RustEmbed;
use warpui::{platform, AddWindowOptions, AssetProvider};

#[derive(Clone, Copy, RustEmbed)]
#[folder = "assets"]
struct Assets;

static ASSETS: Assets = Assets;

impl AssetProvider for Assets {
    fn get(&self, path: &str) -> Result<Cow<'_, [u8]>> {
        <Assets as RustEmbed>::get(path)
            .map(|asset| asset.data)
            .ok_or_else(|| anyhow!("asset not found: {path}"))
    }
}

fn main() -> Result<()> {
    if !graphical_session_available() {
        print_missing_graphical_session();
        std::process::exit(3);
    }

    let binding = aicore_bridge::bind_current_instance().context("绑定 AICore 实例失败")?;
    let title = format!("AICore OS - {}", binding.instance_id);
    let app_builder =
        platform::AppBuilder::new(platform::AppCallbacks::default(), Box::new(ASSETS), None);

    app_builder
        .run(move |ctx| {
            ctx.add_window(
                AddWindowOptions {
                    title: Some(title),
                    window_instance: Some("aicore-tui-warp".to_string()),
                    ..Default::default()
                },
                root_view::RootView::new(binding),
            );
        })
        .map_err(|error| anyhow!("{error:#}"))
}

fn graphical_session_available() -> bool {
    ["WAYLAND_DISPLAY", "WAYLAND_SOCKET", "DISPLAY"]
        .iter()
        .any(|name| env_value_present(name))
}

fn env_value_present(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| !value.is_empty())
}

fn print_missing_graphical_session() {
    eprintln!("AICore TUI Warp UI 无法启动：当前环境没有图形会话。");
    eprintln!("需要设置 WAYLAND_DISPLAY、WAYLAND_SOCKET 或 DISPLAY 后再启动。");
    eprintln!("如果你在 SSH / headless shell 中运行，请先进入图形桌面会话，或等待后续终端 fallback 接入。");
}
