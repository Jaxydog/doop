use std::convert::Infallible;
use std::ops::{ControlFlow, Deref, DerefMut, FromResidual, Residual, Try};

/// Extension of a result, possibly being fatal.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventResult<T = (), E = anyhow::Error> {
    /// Contains the success value.
    Ok(T),
    /// Contains the error value.
    Err(E),
    /// Contains the fatal error value.
    Fatal(E),
}

impl<T, E> EventResult<T, E> {
    /// Returns whether the variant is [`EventResult::Ok`].
    #[inline]
    pub const fn is_ok(&self) -> bool {
        matches!(self, &Self::Ok(_))
    }

    /// Returns whether the variant is [`EventResult::Err`].
    #[inline]
    pub const fn is_err(&self) -> bool {
        matches!(self, &Self::Err(_))
    }

    /// Returns whether the variant is [`EventResult::Fatal`].
    #[inline]
    pub const fn is_fatal(&self) -> bool {
        matches!(self, &Self::Fatal(_))
    }

    /// Returns whether the variant is [`EventResult::Ok`] and matches the
    /// provided predicate.
    #[inline]
    #[must_use]
    #[allow(clippy::wrong_self_convention)]
    pub fn is_ok_and(self, f: impl FnOnce(T) -> bool) -> bool {
        match self {
            Self::Ok(v) => f(v),
            _ => false,
        }
    }

    /// Returns whether the variant is [`EventResult::Err`] and matches the
    /// provided predicate.
    #[inline]
    #[must_use]
    #[allow(clippy::wrong_self_convention)]
    pub fn is_err_and(self, f: impl FnOnce(E) -> bool) -> bool {
        match self {
            Self::Err(v) => f(v),
            _ => false,
        }
    }

    /// Returns whether the variant is [`EventResult::Fatal`] and matches the
    /// provided predicate.
    #[inline]
    #[must_use]
    #[allow(clippy::wrong_self_convention)]
    pub fn is_fatal_and(self, f: impl FnOnce(E) -> bool) -> bool {
        match self {
            Self::Fatal(v) => f(v),
            _ => false,
        }
    }

    /// Converts from [`EventResult<T, E>`] to [`Option<T>`].
    #[inline]
    #[allow(clippy::missing_const_for_fn)]
    pub fn ok(self) -> Option<T> {
        match self {
            Self::Ok(v) => Some(v),
            _ => None,
        }
    }

    /// Converts from [`EventResult<T, E>`] to [`Option<E>`].
    #[inline]
    #[allow(clippy::missing_const_for_fn)]
    pub fn err(self) -> Option<E> {
        match self {
            Self::Err(v) => Some(v),
            _ => None,
        }
    }

    /// Converts from [`EventResult<T, E>`] to [`Option<E>`].
    #[inline]
    #[allow(clippy::missing_const_for_fn)]
    pub fn fatal(self) -> Option<E> {
        match self {
            Self::Fatal(v) => Some(v),
            _ => None,
        }
    }

    /// Converts from [`&EventResult<T, E>`](<EventResult>) to [`EventResult<&T,
    /// &E>`].
    #[inline]
    pub const fn as_ref(&self) -> EventResult<&T, &E> {
        match *self {
            Self::Ok(ref v) => EventResult::Ok(v),
            Self::Err(ref v) => EventResult::Err(v),
            Self::Fatal(ref v) => EventResult::Fatal(v),
        }
    }

    /// Converts from [`&mut EventResult<T, E>`](<EventResult>) to
    /// [`EventResult<&mut T, &mut E>`].
    #[inline]
    pub fn as_mut(&mut self) -> EventResult<&mut T, &mut E> {
        match *self {
            Self::Ok(ref mut v) => EventResult::Ok(v),
            Self::Err(ref mut v) => EventResult::Err(v),
            Self::Fatal(ref mut v) => EventResult::Fatal(v),
        }
    }

    /// Converts from `&EventResult<T, E>` to `EventResult<&<T as
    /// Deref>::Target, &E>`.
    #[inline]
    pub fn as_deref(&self) -> EventResult<&T::Target, &E>
    where
        T: Deref,
    {
        self.as_ref().map(Deref::deref)
    }

    /// Converts from `&mut EventResult<T, E>` to `EventResult<&mut <T as
    /// DerefMut>::Target, &mut E>`.
    #[inline]
    pub fn as_deref_mut(&mut self) -> EventResult<&mut T::Target, &mut E>
    where
        T: DerefMut,
    {
        self.as_mut().map(DerefMut::deref_mut)
    }

    /// Maps a [`EventResult<T, E>`] to [`EventResult<U, E>`] using the
    /// provided function.
    #[inline]
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> EventResult<U, E> {
        match self {
            Self::Ok(v) => EventResult::Ok(f(v)),
            Self::Err(v) => EventResult::Err(v),
            Self::Fatal(v) => EventResult::Fatal(v),
        }
    }

    /// Maps a [`EventResult<T, E>`] to `U` using the provided function and
    /// default.
    #[inline]
    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, f: F) -> U {
        match self {
            Self::Ok(v) => f(v),
            _ => default,
        }
    }

    /// Maps a [`EventResult<T, E>`] to `U` using the provided functions.
    #[inline]
    pub fn map_or_else<U, D: FnOnce(E) -> U, F: FnOnce(T) -> U>(self, d: D, f: F) -> U {
        match self {
            Self::Ok(v) => f(v),
            Self::Err(v) | Self::Fatal(v) => d(v),
        }
    }

    /// Maps a [`EventResult<T, E>`] to [`EventResult<T, U>`] using the
    /// provided function.
    #[inline]
    pub fn map_err<U, F: FnOnce(E) -> U>(self, f: F) -> EventResult<T, U> {
        match self {
            Self::Ok(v) => EventResult::Ok(v),
            Self::Err(v) => EventResult::Err(f(v)),
            Self::Fatal(v) => EventResult::Fatal(f(v)),
        }
    }

    /// Returns `result` if the result is [`EventResult::Ok`], otherwise
    /// returns the inner error.
    #[inline]
    #[allow(clippy::missing_const_for_fn)]
    pub fn and<U>(self, result: EventResult<U, E>) -> EventResult<U, E> {
        match self {
            Self::Ok(_) => result,
            Self::Err(v) => EventResult::Err(v),
            Self::Fatal(v) => EventResult::Fatal(v),
        }
    }

    /// Returns the value provided by `f` if the result is [`EventResult::Ok`],
    /// otherwise returns the inner error.
    #[inline]
    pub fn and_then<U>(self, f: impl FnOnce(T) -> EventResult<U, E>) -> EventResult<U, E> {
        match self {
            Self::Ok(v) => f(v),
            Self::Err(v) => EventResult::Err(v),
            Self::Fatal(v) => EventResult::Fatal(v),
        }
    }

    /// Returns `result` if the result is not [`EventResult::Ok`], otherwise
    /// returns the inner value.
    #[inline]
    #[allow(clippy::missing_const_for_fn)]
    pub fn or<U>(self, result: EventResult<T, U>) -> EventResult<T, U> {
        match self {
            Self::Ok(v) => EventResult::Ok(v),
            Self::Err(_) | Self::Fatal(_) => result,
        }
    }

    /// Returns the value provided by `f` if the result is not
    /// [`EventResult::Ok`], otherwise returns the inner value.
    #[inline]
    #[allow(clippy::missing_const_for_fn)]
    pub fn or_else<U>(self, f: impl FnOnce(E) -> EventResult<T, U>) -> EventResult<T, U> {
        match self {
            Self::Ok(v) => EventResult::Ok(v),
            Self::Err(v) | Self::Fatal(v) => f(v),
        }
    }

    /// Returns the contained [`EventResult::Ok`] value or the provided default.
    #[inline]
    #[allow(clippy::missing_const_for_fn)]
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Self::Ok(v) => v,
            Self::Err(_) | Self::Fatal(_) => default,
        }
    }

    /// Returns the contained [`EventResult::Ok`] value or the value provided
    /// by `f`.
    #[inline]
    #[allow(clippy::missing_const_for_fn)]
    pub fn unwrap_or_else(self, f: impl FnOnce(E) -> T) -> T {
        match self {
            Self::Ok(v) => v,
            Self::Err(v) | Self::Fatal(v) => f(v),
        }
    }
}

impl<T, E> EventResult<&T, E> {
    /// Maps a [`EventResult<&T, E>`] to a [`EventResult<T, E>`] by cloning
    /// the inner value.
    pub fn cloned(self) -> EventResult<T, E>
    where
        T: Clone,
    {
        self.map(std::clone::Clone::clone)
    }

    /// Maps a [`EventResult<&T, E>`] to a [`EventResult<T, E>`] by copying
    /// the inner value.
    pub fn copied(self) -> EventResult<T, E>
    where
        T: Copy,
    {
        self.map(|&t| t)
    }
}

impl<T, E> EventResult<&mut T, E> {
    /// Maps a [`EventResult<&T, E>`] to a [`EventResult<T, E>`] by cloning
    /// the inner value.
    pub fn cloned(self) -> EventResult<T, E>
    where
        T: Clone,
    {
        self.map(|t| t.clone())
    }

    /// Maps a [`EventResult<&T, E>`] to a [`EventResult<T, E>`] by copying
    /// the inner value.
    pub fn copied(self) -> EventResult<T, E>
    where
        T: Copy,
    {
        self.map(|&mut t| t)
    }
}

impl<T, E> EventResult<Option<T>, E> {
    /// Transposes a [`EventResult`] of an [`Option`] into an [`Option`] of a
    /// [`EventResult`].
    #[allow(clippy::missing_const_for_fn)]
    pub fn transpose(self) -> Option<EventResult<T, E>> {
        match self {
            Self::Ok(Some(v)) => Some(EventResult::Ok(v)),
            Self::Ok(None) => None,
            Self::Err(v) => Some(EventResult::Err(v)),
            Self::Fatal(v) => Some(EventResult::Fatal(v)),
        }
    }
}

impl<T, E> EventResult<EventResult<T, E>, E> {
    /// Flattens the handle result.
    #[inline]
    pub fn flatten(self) -> EventResult<T, E> {
        self.and_then(std::convert::identity)
    }
}

impl<T, E> From<EventResult<T, E>> for Result<T, E> {
    fn from(value: EventResult<T, E>) -> Self {
        match value {
            EventResult::Ok(v) => Ok(v),
            EventResult::Err(v) | EventResult::Fatal(v) => Err(v),
        }
    }
}

impl<T, E> From<Result<T, E>> for EventResult<T, E> {
    fn from(value: Result<T, E>) -> Self {
        match value {
            Ok(v) => Self::Ok(v),
            Err(v) => Self::Err(v),
        }
    }
}

impl<T, E> Residual<T> for EventResult<Infallible, E> {
    type TryType = EventResult<T, E>;
}

impl<T, E, F: From<E>> FromResidual<Result<Infallible, E>> for EventResult<T, F> {
    fn from_residual(residual: Result<Infallible, E>) -> Self {
        match residual {
            Ok(_) => unreachable!(),
            Err(v) => Self::Err(From::from(v)),
        }
    }
}

impl<T, E, F: From<E>> FromResidual<EventResult<Infallible, E>> for EventResult<T, F> {
    #[inline]
    #[track_caller]
    fn from_residual(residual: EventResult<Infallible, E>) -> Self {
        match residual {
            EventResult::Ok(_) => unreachable!(),
            EventResult::Err(v) => Self::Err(From::from(v)),
            EventResult::Fatal(v) => Self::Fatal(From::from(v)),
        }
    }
}

impl<T, E> Try for EventResult<T, E> {
    type Output = T;
    type Residual = EventResult<Infallible, E>;

    #[inline]
    fn from_output(output: Self::Output) -> Self {
        Self::Ok(output)
    }

    #[inline]
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            Self::Ok(v) => ControlFlow::Continue(v),
            Self::Err(v) => ControlFlow::Break(EventResult::Err(v)),
            Self::Fatal(v) => ControlFlow::Break(EventResult::Fatal(v)),
        }
    }
}
