/// Contains API response macros.
mod api;
/// Contains command definition macros.
mod cmd;

/// Wraps the given value in a heap-allocated smart pointer.
///
/// Possible pointers are: `box`, `rc`, and `arc`.
///
/// # Examples
/// ```
/// let value = heap!(box [1, 2, 3, 4, 5]);
/// let boxed = Box::new([1, 2, 3, 4, 5]);
///
/// assert_eq!(value, boxed);
/// ```
#[macro_export]
macro_rules! heap {
    (box $value:expr) => {
        ::std::boxed::Box::new($value)
    };
    (rc $value:expr) => {
        ::std::rc::Rc::new($value)
    };
    (arc $value:expr) => {
        ::std::sync::Arc::new($value)
    };
}

/// Creates a once-locked static value with an associated getter function.
///
/// # Examples
/// ```
/// // Will initialize the `OnceLock` using `Vec::default`
/// global! {{
///     /// Returns a vector.
///     [VECTOR] fn vector() -> Vec<u8>;
/// }}
///
/// assert!(vector().is_empty());
/// ```
///
/// This can also be called with a defined initializer.
///
/// ```
/// global! {{
///     /// Returns the logger instance.
///     [VECTOR] fn vector() -> Vec<u8> { || vec![1, 2, 3] }
/// }}
///
/// assert!(vector().len() == 3);
/// ```
#[macro_export]
macro_rules! global {
    {$({$($args:tt)+})*} => {$(
        $crate::global!(@match { $($args)+ });
    )*};
    (@match {
        $(#[$attribute:meta])*
        [$global:ident] fn $fn:ident() -> $type:ty;
    }) => {
        $crate::global!(@build($global, ($($attribute)*), $fn, $type, <$type>::default));
    };
    (@match {
        $(#[$attribute:meta])*
        [$global:ident] fn $fn:ident() -> $type:ty { $init:expr }
    }) => {
        $crate::global!(@build($global, ($($attribute)*), $fn, $type, $init));
    };
    (@build($global:ident, ($($attribute:meta)*), $fn:ident, $type:ty, $init:expr)) => {
        static $global: ::std::sync::OnceLock<$type> = ::std::sync::OnceLock::new();

        $(#[$attribute])*
        #[inline]
        #[must_use]
        pub fn $fn() -> &'static $type {
            $global.get_or_init($init)
        }
    };
}
