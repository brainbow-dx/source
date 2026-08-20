//! Plain terminal-width text wrapping — no `Scaffold`/tracing/UI-state involvement at all, just
//! string-in, wrapped-lines-out. Extracted from `apps/anvil`'s `main.rs`: nothing here is
//! Anvil-specific, it belongs in this library, not the app.

use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

/// Word-wraps `text` to `width` columns, with `gutter` prepended to the first line and blank
/// padding of the same display width prepended to every line after — i.e. a hanging indent.
pub fn wrap_hanging(text: &str, gutter: &str, width: usize) -> String {
    let indent_width = UnicodeWidthStr::width(gutter);
    let content_width = width.saturating_sub(indent_width).max(1);

    wrap_words(text, content_width)
        .into_iter()
        .enumerate()
        .map(|(i, line)| if i == 0 { format!("{}{}", gutter, line) } else { format!("{}{}", " ".repeat(indent_width), line) })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Greedy word-wrap at `width` display columns — breaks at whitespace where possible, and
/// hard-breaks any single token wider than `width` on its own (mirrors what `Paragraph::wrap`
/// does for unbreakable tokens, e.g. a long path with no spaces). Doesn't add any indentation
/// itself; see `wrap_hanging` for that.
pub fn wrap_words(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();

    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut current = String::new();
        let mut current_width = 0usize;

        for word in paragraph.split(' ') {
            let word_width = UnicodeWidthStr::width(word);

            if word_width > width {
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                    current_width = 0;
                }
                lines.extend(hard_break(word, width));
                continue;
            }

            if current.is_empty() {
                current.push_str(word);
                current_width = word_width;
            } else if current_width + 1 + word_width <= width {
                current.push(' ');
                current.push_str(word);
                current_width += 1 + word_width;
            } else {
                lines.push(std::mem::take(&mut current));
                current.push_str(word);
                current_width = word_width;
            }
        }

        lines.push(current);
    }

    lines
}

/// Strips ANSI CSI escape sequences (`\x1b[...<final byte>`, e.g. `owo_colors`' own SGR color
/// codes) from `text`, leaving plain characters and `\n` intact. Needed because nested/wrapped
/// ANSI color codes don't compose in a real terminal — recoloring already-colored text uniformly
/// (e.g. dimming a transcript that already carries its own per-message colors) only works if its
/// existing color is stripped first, not wrapped in another color on top.
pub fn strip_ansi_codes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();

    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            let mut lookahead = chars.clone();
            if lookahead.next() == Some('[') {
                chars = lookahead;
                for next in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&next) {
                        break;
                    }
                }
                continue;
            }
        }
        out.push(c);
    }

    out
}

/// Pads or truncates a single line (no `\n`) to exactly `width` display columns — real column
/// math (`UnicodeWidthChar`), not byte/char count, so wide characters aren't miscounted.
pub fn pad_to_width(text: &str, width: usize) -> String {
    let mut out = String::new();
    let mut out_width = 0usize;

    for c in text.chars() {
        let char_width = UnicodeWidthChar::width(c).unwrap_or(0);
        if out_width + char_width > width {
            break;
        }
        out.push(c);
        out_width += char_width;
    }

    out.push_str(&" ".repeat(width.saturating_sub(out_width)));
    out
}

/// For a single-line, priority-ordered list of segments (highest priority first, `widths[i]` the
/// already-measured plain display width of segment `i`), returns how many of the *leading*
/// segments fit together within `max_width` once joined by a separator of `separator_width`
/// columns. Deliberately simple — "keep going until the next one doesn't fit, then stop," not a
/// bin-packing search that might skip a low-priority segment to make room for a later one — which
/// is exactly the desired behavior for something like a status bar: drop the least important
/// information first (the tail of the list) as the terminal narrows, never reorder what's shown.
pub fn fit_segment_count_by_priority(widths: &[usize], separator_width: usize, max_width: usize) -> usize {
    let mut total = 0usize;
    let mut count = 0usize;

    for (i, &width) in widths.iter().enumerate() {
        let candidate = total + width + if i == 0 { 0 } else { separator_width };
        if candidate > max_width {
            break;
        }
        total = candidate;
        count += 1;
    }

    count
}

/// Splits a single unbreakable token into `width`-wide chunks, for the case a word alone is
/// too long to fit any line no matter what (e.g. a long path, or a long unbroken test string).
pub fn hard_break(word: &str, width: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;

    for c in word.chars() {
        let char_width = UnicodeWidthChar::width(c).unwrap_or(0);
        if current_width + char_width > width && !current.is_empty() {
            chunks.push(std::mem::take(&mut current));
            current_width = 0;
        }
        current.push(c);
        current_width += char_width;
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}
