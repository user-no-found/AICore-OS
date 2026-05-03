use aicore_bridge::AicoreWarpBinding;
use pathfinder_color::ColorU;
use warpui::fonts::FamilyId;
use warpui::SingletonEntity as _;
use warpui::{
    elements::{
        Align, Border, ConstrainedBox, Container, CrossAxisAlignment, Expanded, Flex, MainAxisSize,
        Padding, ParentElement, Rect, Shrinkable, Stack, Text,
    },
    AppContext, Element, Entity, TypedActionView, View, ViewContext,
};

pub struct RootView {
    binding: AicoreWarpBinding,
    font_family: FamilyId,
}

impl RootView {
    pub fn new(binding: AicoreWarpBinding) -> impl FnOnce(&mut ViewContext<Self>) -> Self {
        move |ctx| {
            let font_family = warpui::fonts::Cache::handle(ctx).update(ctx, |cache, _| {
                cache
                    .load_system_font("JetBrains Mono")
                    .or_else(|_| cache.load_system_font("Menlo"))
                    .or_else(|_| cache.load_system_font("Arial"))
                    .unwrap_or(FamilyId(0))
            });
            Self {
                binding,
                font_family,
            }
        }
    }

    fn text(&self, value: impl Into<String>, size: f32, color: ColorU) -> Box<dyn Element> {
        Text::new(value.into(), self.font_family, size)
            .with_color(color)
            .finish()
    }

    fn label(&self, value: impl Into<String>) -> Box<dyn Element> {
        self.text(value, 13.0, ColorU::new(143, 153, 168, 255))
    }

    fn value(&self, value: impl Into<String>) -> Box<dyn Element> {
        self.text(value, 14.0, ColorU::new(231, 236, 244, 255))
    }

    fn mono_line(&self, value: impl Into<String>) -> Box<dyn Element> {
        self.text(value, 13.0, ColorU::new(190, 199, 212, 255))
    }

    fn info_row(&self, label: impl Into<String>, value: impl Into<String>) -> Box<dyn Element> {
        Flex::row()
            .with_spacing(18.0)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                ConstrainedBox::new(self.label(label))
                    .with_width(110.0)
                    .finish(),
            )
            .with_child(Expanded::new(1.0, self.value(value)).finish())
            .finish()
    }

    fn pill(&self, value: impl Into<String>, color: ColorU) -> Box<dyn Element> {
        Container::new(self.text(value, 12.0, ColorU::new(236, 241, 248, 255)))
            .with_padding(Padding::uniform(8.0).with_horizontal(12.0))
            .with_background_color(color)
            .with_border(Border::all(1.0).with_border_color(ColorU::new(255, 255, 255, 36)))
            .finish()
    }

    fn panel(&self, child: Box<dyn Element>) -> Box<dyn Element> {
        Container::new(child)
            .with_padding(Padding::uniform(22.0))
            .with_background_color(ColorU::new(20, 25, 34, 232))
            .with_border(Border::all(1.0).with_border_color(ColorU::new(82, 99, 124, 130)))
            .finish()
    }

    fn sidebar(&self) -> Box<dyn Element> {
        self.panel(
            Flex::column()
                .with_spacing(18.0)
                .with_child(self.text("AICore OS", 26.0, ColorU::new(249, 251, 255, 255)))
                .with_child(self.text("WarpUI Shell", 14.0, ColorU::new(116, 204, 255, 255)))
                .with_child(self.info_row("实例", self.binding.instance_id.clone()))
                .with_child(self.info_row("类型", self.binding.instance_kind.clone()))
                .with_child(
                    self.info_row("工作区", self.binding.workspace_root.display().to_string()),
                )
                .with_child(
                    self.info_row("状态目录", self.binding.instance_root.display().to_string()),
                )
                .with_child(
                    Flex::row()
                        .with_spacing(8.0)
                        .with_child(self.pill("UI-only", ColorU::new(43, 92, 130, 255)))
                        .with_child(self.pill("No Agent", ColorU::new(100, 67, 38, 255)))
                        .finish(),
                )
                .finish(),
        )
    }

    fn transcript(&self) -> Box<dyn Element> {
        self.panel(
            Flex::column()
                .with_spacing(18.0)
                .with_child(self.text("会话显示区", 20.0, ColorU::new(250, 252, 255, 255)))
                .with_child(self.mono_line("TUI 已绑定当前文件夹对应的 AICore instance。"))
                .with_child(self.mono_line("当前只运行界面层，不启动智能体运行时。"))
                .with_child(
                    self.mono_line(
                        "后续消息流、停止、审批、记忆提案会通过 AICore Unified I/O 接入。",
                    ),
                )
                .with_child(
                    Container::new(
                        Flex::column()
                            .with_spacing(10.0)
                            .with_child(self.text(
                                "等待输入",
                                14.0,
                                ColorU::new(172, 232, 190, 255),
                            ))
                            .with_child(
                                self.mono_line("这里会成为 TUI / Web 共享的 instance 输出镜像。"),
                            )
                            .finish(),
                    )
                    .with_padding(Padding::uniform(16.0))
                    .with_background_color(ColorU::new(8, 12, 18, 235))
                    .with_border(Border::all(1.0).with_border_color(ColorU::new(56, 83, 116, 180)))
                    .finish(),
                )
                .finish(),
        )
    }

    fn composer(&self) -> Box<dyn Element> {
        Container::new(
            Flex::row()
                .with_spacing(14.0)
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(self.text("输入", 13.0, ColorU::new(119, 205, 255, 255)))
                .with_child(
                    Expanded::new(
                        1.0,
                        Container::new(self.mono_line("Unified I/O 未接入，当前为界面占位。"))
                            .with_padding(Padding::uniform(13.0).with_horizontal(16.0))
                            .with_background_color(ColorU::new(7, 10, 15, 245))
                            .with_border(
                                Border::all(1.0).with_border_color(ColorU::new(62, 80, 105, 220)),
                            )
                            .finish(),
                    )
                    .finish(),
                )
                .with_child(self.pill("Enter 发送", ColorU::new(33, 88, 72, 255)))
                .with_child(self.pill("Esc 停止", ColorU::new(92, 51, 46, 255)))
                .finish(),
        )
        .with_padding(Padding::uniform(16.0))
        .with_background_color(ColorU::new(15, 19, 27, 242))
        .with_border(Border::all(1.0).with_border_color(ColorU::new(71, 90, 116, 150)))
        .finish()
    }
}

impl Entity for RootView {
    type Event = ();
}

impl View for RootView {
    fn ui_name() -> &'static str {
        "AicoreWarpRootView"
    }

    fn render(&self, _: &AppContext) -> Box<dyn Element> {
        Stack::new()
            .with_child(
                Rect::new()
                    .with_background_color(ColorU::new(5, 8, 13, 255))
                    .finish(),
            )
            .with_child(
                Align::new(
                    Container::new(
                        Flex::column()
                            .with_spacing(18.0)
                            .with_main_axis_size(MainAxisSize::Max)
                            .with_child(
                                Expanded::new(
                                    1.0,
                                    Flex::row()
                                        .with_spacing(18.0)
                                        .with_main_axis_size(MainAxisSize::Max)
                                        .with_child(
                                            ConstrainedBox::new(self.sidebar())
                                                .with_width(380.0)
                                                .finish(),
                                        )
                                        .with_child(Expanded::new(1.0, self.transcript()).finish())
                                        .finish(),
                                )
                                .finish(),
                            )
                            .with_child(Shrinkable::new(0.0, self.composer()).finish())
                            .finish(),
                    )
                    .with_padding(Padding::uniform(28.0))
                    .finish(),
                )
                .finish(),
            )
            .finish()
    }
}

impl TypedActionView for RootView {
    type Action = ();
}
