pub fn display_width(value: &str) -> usize {
    value.chars().map(char_width).sum()
}

pub fn fit(value: &str, width: usize) -> String {
    let value = value.replace('\n', " ");
    let mut text = if display_width(&value) > width {
        let mut text = truncate_display(&value, width.saturating_sub(1));
        text.push('…');
        text
    } else {
        value
    };
    text = truncate_display(&text, width);
    let padding = width.saturating_sub(display_width(&text));
    format!("{text}{}", " ".repeat(padding))
}

pub fn wrap(value: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }
    let mut rows = Vec::new();
    let mut current = String::new();
    for ch in value.chars() {
        if display_width(&current) + char_width(ch) > width {
            rows.push(current);
            current = String::new();
        }
        current.push(ch);
    }
    if current.is_empty() {
        rows.push(String::new());
    } else {
        rows.push(current);
    }
    rows
}

pub fn truncate_display(value: &str, width: usize) -> String {
    let mut out = String::new();
    let mut used = 0;
    for ch in value.chars() {
        let char_width = char_width(ch);
        if used + char_width > width {
            break;
        }
        out.push(ch);
        used += char_width;
    }
    out
}

fn char_width(ch: char) -> usize {
    let value = ch as u32;
    if (0x1100..=0x115f).contains(&value)
        || (0x2e80..=0xa4cf).contains(&value)
        || (0xac00..=0xd7a3).contains(&value)
        || (0xf900..=0xfaff).contains(&value)
        || (0xfe10..=0xfe19).contains(&value)
        || (0xfe30..=0xfe6f).contains(&value)
        || (0xff00..=0xff60).contains(&value)
        || (0xffe0..=0xffe6).contains(&value)
    {
        2
    } else {
        1
    }
}
