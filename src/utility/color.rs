use std::fmt::Display;

use serde::{Deserialize, Serialize};

/// The bot's branding color.
pub const BRANDING_COLOR: Color = Color::new(0x24, 0x9F, 0xDE);
/// The bot's success color.
pub const SUCCESS_COLOR: Color = Color::new(0x59, 0xC1, 0x35);
/// The bot's failure color.
pub const FAILURE_COLOR: Color = Color::new(0xB4, 0x20, 0x2A);

/// Defines a structure that represents an RGB color.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color(u8, u8, u8);

impl Color {
    /// Creates a new RGB color value.
    #[inline]
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self { Self(r, g, b) }

    /// Returns the color's R component.
    #[inline]
    #[must_use]
    pub const fn r(&self) -> &u8 { &self.0 }

    /// Returns the color's G component.
    #[inline]
    #[must_use]
    pub const fn g(&self) -> &u8 { &self.0 }

    /// Returns the color's B component.
    #[inline]
    #[must_use]
    pub const fn b(&self) -> &u8 { &self.0 }
}

impl From<u32> for Color {
    fn from(value: u32) -> Self {
        let r = ((value >> 16) & 0xFF) as u8;
        let g = ((value >> 8) & 0xFF) as u8;
        let b = ((value) & 0xFF) as u8;

        Self::new(r, g, b)
    }
}

impl From<(u8, u8, u8)> for Color {
    fn from((r, g, b): (u8, u8, u8)) -> Self { Self::new(r, g, b) }
}

impl From<[u8; 3]> for Color {
    fn from([r, g, b]: [u8; 3]) -> Self { Self::new(r, g, b) }
}

impl From<Color> for u32 {
    fn from(value: Color) -> Self {
        let Color(r, g, b) = value;

        let r = Self::from(r) << 16;
        let g = Self::from(g) << 8;
        r | g | Self::from(b)
    }
}

impl From<Color> for (u8, u8, u8) {
    fn from(Color(r, g, b): Color) -> Self { (r, g, b) }
}

impl From<Color> for [u8; 3] {
    fn from(Color(r, g, b): Color) -> Self { [r, g, b] }
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rgb({}, {}, {})", self.r(), self.g(), self.b())
    }
}
