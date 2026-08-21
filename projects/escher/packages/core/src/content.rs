use num_traits::Num;
use parking_lot::RwLock;

use unicode_width::UnicodeWidthChar;

use alloc::sync::Arc;

use core::fmt::Debug;
use core::fmt::Display;
use core::fmt::Error as FmtError;
use core::fmt::Formatter;

pub mod prelude {
    pub use super::Content;
    pub use super::LineCounter;
}

pub trait Content: Debug {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError>;
}

impl<C: Content> Content for &C {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Content::fmt(*self, f)
    }
}

impl Display for &dyn Content {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Content::fmt(*self, f)
    }
}

impl Content for &str {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        f.write_str(self)
    }
}

impl Content for String {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        f.write_str(self.as_str())
    }
}

impl Content for Arc<RwLock<String>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        f.write_str(self.read().as_str())
    }
}

impl Content for Option<Arc<RwLock<String>>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        match self {
            Some(content) => f.write_str(content.read().as_str()),
            None => f.write_str("<empty>"),
        }
    }
}

//--
/// A writer that counts the terminal rows needed to render text without allocating memory,
/// accounting for both hard newlines and soft (word-wrap) breaks at a fixed column width.
pub struct LineCounter<T> {
    /// The number of completed rows counted so far.
    rows: T,
    /// Display-width columns already consumed on the current, unterminated row.
    current_width: usize,
    /// The column width a row wraps at. `0` disables wrapping (only `\n` is counted).
    wrap_width: usize,
    /// Tracks if any characters have been written at all.
    is_empty: bool,
}

impl<T: Default> LineCounter<T> {
    /// Creates a new LineCounter that soft-wraps at `wrap_width` columns.
    pub fn new(wrap_width: usize) -> Self {
        LineCounter {
            rows: T::default(),
            current_width: 0,
            wrap_width,
            is_empty: true,
        }
    }
}

impl<T: Num + Copy> LineCounter<T> {
    /// Calculates the final row count, including any partially-written trailing row.
    /// An empty string has 0 rows.
    pub fn count(&self) -> T {
        if self.is_empty {
            T::zero()
        } else if self.current_width > 0 {
            self.rows + T::one()
        } else {
            self.rows
        }
    }
}

/// Display width of `s`, skipping ANSI CSI escape sequences (e.g. SGR color codes) — content
/// rendered through `ansi_to_tui` may legitimately contain them, and they take up zero columns.
pub fn display_width(s: &str) -> usize {
    let mut width = 0;
    let mut chars = s.chars();

    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.next() == Some('[') {
                for next in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&next) {
                        break;
                    }
                }
            }
            continue;
        }

        width += UnicodeWidthChar::width(c).unwrap_or(0);
    }

    width
}

impl<T: Num + Copy> core::fmt::Write for LineCounter<T> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if !s.is_empty() {
            self.is_empty = false;
        }

        let mut segments = s.split('\n').peekable();
        while let Some(segment) = segments.next() {
            self.current_width += display_width(segment);

            if self.wrap_width > 0 {
                while self.current_width > self.wrap_width {
                    self.current_width -= self.wrap_width;
                    self.rows = self.rows + T::one();
                }
            }

            // A `\n` followed this segment: the row ends here regardless of width.
            if segments.peek().is_some() {
                self.rows = self.rows + T::one();
                self.current_width = 0;
            }
        }

        Ok(())
    }
}
