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
