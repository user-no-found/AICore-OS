pub fn display_width(value: &str) -> usize {
    value
        .chars()
        .map(|ch| if ch as u32 >= 0x1100 { 2 } else { 1 })
        .sum()
}

pub(crate) fn pad_display(value: &str, target_width: usize) -> String {
    let width = display_width(value);
    if width >= target_width {
        value.to_string()
    } else {
        format!("{}{}", value, " ".repeat(target_width - width))
    }
}
