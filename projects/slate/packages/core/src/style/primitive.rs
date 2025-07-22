use core::fmt::Debug;
use core::fmt::Display;

// use alloc::vec::Vec;
// use alloc::format;

//---
#[derive(Default, Debug, PartialEq)]
pub struct Weight<P>(pub P)
where
    P: Display + Debug + PartialEq;

/// TODO
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub enum Unit<P = f32>
where
    P: Display + Debug + PartialEq,
{
    /// TODO
    Px(P),

    /// TOD=
    Percent(P),

    /// TODO
    Full,

    /// TODO
    Auto,

    /// TODO
    #[default]
    None,
}

impl<P> Display for Unit<P>
where
    P: Display + Debug + PartialEq,
{
    /// TODO
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Unit::Px(value) => write!(f, "{}px", value),
            Unit::Percent(value) => write!(f, "{}%", value),
            Unit::Full => write!(f, "full"),
            Unit::Auto => write!(f, "auto"),
            Unit::None => write!(f, "none"),
        }
    }
}

impl From<f32> for Unit<f32> {
    /// TODO
    fn from(value: f32) -> Self {
        Unit::Px(value)
    }
}

impl<P> From<Option<P>> for Unit<P>
where
    P: Display + Debug + PartialEq,
{
    /// TODO
    fn from(value: Option<P>) -> Self {
        match value {
            Some(value) => Unit::Px(value),
            None => Unit::None,
        }
    }
}

/// TODO
#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct Size2d<P = f32>(pub(crate) Unit<P>, pub(crate) Unit<P>)
where
    P: Display + Debug + PartialEq;

impl<P> Size2d<P>
where
    P: Display + Debug + PartialEq,
{
    /// TODO
    pub fn xy<U1: Into<Unit<P>>, U2: Into<Unit<P>>>(x: U1, y: U2) -> Self {
        Size2d(x.into(), y.into())
    }

    /// TODO
    pub fn x(&self) -> &Unit<P> {
        &self.0
    }

    /// TODO
    pub fn y(&self) -> &Unit<P> {
        &self.1
    }
}

/// TODO
#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct Rect<P = f32>(
    pub(crate) Unit<P>,
    pub(crate) Unit<P>,
    pub(crate) Unit<P>,
    pub(crate) Unit<P>,
)
where
    P: Display + Debug + PartialEq;

impl<P> Rect<P>
where
    P: Copy + Display + Debug + PartialEq,
{
    /// TODO
    pub fn all<U1: Into<Unit<P>>, U2: Into<Unit<P>>, U3: Into<Unit<P>>, U4: Into<Unit<P>>>(
        top: U1,
        right: U2,
        bottom: U3,
        left: U4,
    ) -> Self {
        Rect(top.into(), right.into(), bottom.into(), left.into())
    }

    /// TODO
    pub fn xy<U1: Into<Unit<P>> + Copy, U2: Into<Unit<P>> + Copy>(x: U1, y: U2) -> Self {
        Rect(x.into(), y.into(), x.into(), y.into())
    }
}

impl<P> Rect<P>
where
    P: Display + Debug + PartialEq,
{
    /// TODO
    pub fn top(&self) -> &Unit<P> {
        &self.0
    }

    /// TODO
    pub fn right(&self) -> &Unit<P> {
        &self.1
    }

    /// TODO
    pub fn bottom(&self) -> &Unit<P> {
        &self.2
    }

    /// TODO
    pub fn left(&self) -> &Unit<P> {
        &self.3
    }
}

/// Represents a color in various formats.
#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub enum Color {
    /// RGBA color format.
    Rgba(u8, u8, u8, u8),

    /// HSLA color format.
    Hsla(f32, f32, f32, f32),

    /// TODO
    #[default]
    Transparent,
}

impl Color {
    /// TODO
    pub fn hex(hex: &str) -> Result<Self, ColorError> {
        Color::decode_hex_color(hex)
    }

    /// TODO
    /// Ref: https://play.rust-lang.org/?version=stable&mode=debug&edition=2015&gist=e241493d100ecaadac3c99f37d0f766f
    pub fn decode_hex_color(hex_utf8: &str) -> Result<Self, ColorError> {
        let mut bytes: [u8; 4] = [0, 0, 0, 255];
        let hex_utf8 = hex_utf8.trim_start_matches('#').as_bytes();

        match hex_utf8.len() {
            6 | 8 => {
                for (i, chunk) in hex_utf8.chunks(2).enumerate() {
                    let chunk = core::str::from_utf8(chunk)?;
                    bytes[i] = u8::from_str_radix(chunk, 16)?;
                }
            }
            input_length => {
                return Err(ColorError::InvalidLength(input_length));
            }
        }

        Ok(Color::Rgba(bytes[0], bytes[1], bytes[2], bytes[3]))
    }
}

/// TODO
#[derive(oops::Error, Clone, derive_more::From)]
pub enum ColorError {
    /// TODO
    #[msg("invalid length '{0}'")]
    InvalidLength(usize),

    /// TODO
    #[msg("parse int error: {0}")]
    ParseIntError(core::num::ParseIntError),

    /// TODO
    #[msg("utf8 error: {0}")]
    Utf8Error(alloc::str::Utf8Error),

    /// TODO
    #[msg("chunk error: {0}")]
    ChunkError(String),
}
