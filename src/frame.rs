use crossterm::style::Color;

use crate::cell::Cell;

/// Draw a window frame (box-drawing characters) into the buffer.
///
/// If `title` is provided and the window is large enough (w ≥ 6, h ≥ 5),
/// draws an extended frame with a title box above a separator:
///
/// ```text
/// ╭─────────╮
/// │ title   │
/// ├─────────┤
/// │ content │
/// │ ...     │
/// └─────────┘
/// ```
///
/// Otherwise falls back to a simple box.
#[allow(clippy::too_many_arguments)]
pub fn draw_frame(
    buf: &mut [Vec<Cell>],
    x: i32,
    y: i32,
    w: u16,
    h: u16,
    title: Option<&str>,
    border_fg: Color,
    screen_rows: u16,
    screen_cols: u16,
) {
    let fg = border_fg;
    let bg = Color::Reset;

    let x_end = x + w as i32;
    let y_end = y + h as i32;

    let max_title = (w as usize).saturating_sub(4);
    let title_display: Option<String> = title.and_then(|t| {
        if w < 6 || h < 5 || t.is_empty() {
            None
        } else if t.len() > max_title {
            Some(t.chars().take(max_title).collect())
        } else {
            Some(t.to_string())
        }
    });
    let show_title = title_display.is_some();

    for row in y..y_end {
        if row < 0 || row >= screen_rows as i32 {
            continue;
        }
        let r = row as usize;
        let is_top = row == y;
        let is_bottom = row == y_end - 1;
        let is_separator = show_title && row == y + 2;
        let is_title_row = show_title && row == y + 1;

        for col in x..x_end {
            if col < 0 || col >= screen_cols as i32 {
                continue;
            }
            let c = col as usize;
            let is_left = col == x;
            let is_right = col == x_end - 1;

            let ch = if is_top {
                if is_left {
                    '╭'
                } else if is_right {
                    '╮'
                } else {
                    '─'
                }
            } else if is_bottom {
                if is_left {
                    '└'
                } else if is_right {
                    '┘'
                } else {
                    '─'
                }
            } else if is_separator {
                if is_left {
                    '├'
                } else if is_right {
                    '┤'
                } else {
                    '─'
                }
            } else if is_title_row {
                if is_left || is_right {
                    '│'
                } else {
                    let inner = (col - x - 1) as usize;
                    let td = title_display.as_deref().unwrap_or("");
                    let pad = max_title.saturating_sub(td.len()) / 2;
                    if inner >= pad && inner < pad + td.len() {
                        td.chars().nth(inner - pad).unwrap_or(' ')
                    } else {
                        ' '
                    }
                }
            } else {
                if is_left || is_right {
                    '│'
                } else {
                    continue;
                }
            };

            buf[r][c] = Cell { ch, fg, bg };
        }
    }
}

/// Draw a centered hint message (used when no windows exist).
pub fn draw_hint(buf: &mut [Vec<Cell>], rows: u16, cols: u16, message: &str, fg: Color) {
    if buf.is_empty() || rows == 0 || cols == 0 {
        return;
    }
    let row = (rows as usize / 2).min(buf.len().saturating_sub(1));
    let col_start = (cols as usize).saturating_sub(message.len()) / 2;
    let bg = Color::Reset;

    for (i, ch) in message.chars().enumerate() {
        let c = col_start + i;
        if c < buf[row].len() {
            buf[row][c] = Cell { ch, fg, bg };
        }
    }
}
