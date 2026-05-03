use crate::state::TuiBlockKind;

pub struct BlockStyle {
    pub marker: &'static str,
    pub label: &'static str,
}

pub fn style_for(kind: &TuiBlockKind) -> BlockStyle {
    match kind {
        TuiBlockKind::Prompt => BlockStyle {
            marker: "›",
            label: "用户",
        },
        TuiBlockKind::Agent => BlockStyle {
            marker: "◆",
            label: "运行",
        },
        TuiBlockKind::Tool => BlockStyle {
            marker: "$",
            label: "工具",
        },
        TuiBlockKind::Approval => BlockStyle {
            marker: "!",
            label: "审批",
        },
        TuiBlockKind::Terminal => BlockStyle {
            marker: "▸",
            label: "事件",
        },
        TuiBlockKind::Assistant => BlockStyle {
            marker: "●",
            label: "系统",
        },
    }
}
