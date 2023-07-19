/// Provides additional methods for [`char`]s
pub trait CharExt {
    /// Creates a new [`String`] by repeating the character `n` times.
    fn repeat(&self, n: usize) -> String;
}

impl CharExt for char {
    #[inline]
    fn repeat(&self, n: usize) -> String { self.encode_utf8(&mut [0; 4]).repeat(n) }
}

/// Provides additional methods for [`str`]s
pub trait StrExt {
    /// Converts basic escape sequences into their actual characters.
    ///
    /// Currently only replaces `\t`, `\n`, and `\r`.
    fn flatten_escapes(&self) -> String;
}

impl StrExt for str {
    #[inline]
    fn flatten_escapes(&self) -> String {
        self.replace(r"\t", "\t")
            .replace(r"\n", "\n")
            .replace(r"\r", "\r")
    }
}
