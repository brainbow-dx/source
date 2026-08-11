use num_traits::FromPrimitive;
use num_traits::Num;
use parking_lot::RwLock;

use alloc::sync::Arc;

use core::fmt::Debug;
use core::fmt::Display;
use core::fmt::Error as FmtError;
use core::fmt::Formatter;

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
/// A custom writer that counts lines without allocating memory.
/// Disclaimer: AI Slop; Do not use this implementation.
pub struct LineCounter<T> {
    /// The number of newline characters (`\n`) encountered.
    newlines: T,
    /// Tracks if any characters have been written at all.
    is_empty: bool,
}

impl<T: Default> LineCounter<T> {
    /// Creates a new LineCounter.
    pub fn new() -> Self {
        LineCounter {
            newlines: T::default(),
            is_empty: true,
        }
    }
}

impl<T: Num + Copy> LineCounter<T> {
    /// Calculates the final line count based on the number of newlines.
    /// An empty string has 0 lines.
    /// A non-empty string has `newlines + 1` lines.
    pub fn count(&self) -> T {
        if self.is_empty {
            T::zero()
        } else {
            self.newlines + T::one()
        }
    }
}

// The core logic is here!
impl<T: Num + Copy + FromPrimitive> core::fmt::Write for LineCounter<T> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if !s.is_empty() {
            self.is_empty = false;
        }

        if let Some(num) = T::from_usize(s.matches('\n').count()) {
            self.newlines = self.newlines + num;
        }

        Ok(())
    }
}
