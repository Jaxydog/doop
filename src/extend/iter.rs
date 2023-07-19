use std::iter::FusedIterator;
use std::ops::ControlFlow;

/// Provides additional methods for all iterators.
pub trait IteratorExt: Iterator {
    /// Applies a function to the elements of the iterator and returns the first
    /// non-error result.
    #[inline]
    fn try_find_map<T, E, F>(&mut self, mut f: F) -> Option<T>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> Result<T, E>,
    {
        #[inline]
        fn find<Item, T, E, F>(f: &mut F) -> impl FnMut((), Item) -> ControlFlow<T> + '_
        where
            F: FnMut(Item) -> Result<T, E>,
        {
            move |(), x| f(x).map_or_else(|_| ControlFlow::Continue(()), ControlFlow::Break)
        }

        self.try_fold((), find(&mut f)).break_value()
    }

    /// Creates an iterator that both filters and maps, yielding values that do
    /// not produce an error.
    fn try_filter_map<T, E, F>(self, f: F) -> TryFilterMap<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> Result<T, E>,
    {
        TryFilterMap::new(self, f)
    }
}

impl<T: Iterator> IteratorExt for T {}

/// An iterator that uses `f` to both filter and map elements from `iter`.
#[must_use = "iterators are lazy and do nothing unless consumed"]
#[derive(Clone)]
pub struct TryFilterMap<I, F> {
    iter: I,
    f: F,
}

impl<I, F> TryFilterMap<I, F> {
    /// Creates a new try filter map.
    #[inline]
    const fn new(iter: I, f: F) -> Self { Self { iter, f } }

    /// Runs a try-fold method combining two fold functions.
    pub(self) fn try_fold<'a, Item, T, E, Acc, Return: std::ops::Try<Output = Acc>>(
        f: &'a mut impl FnMut(Item) -> Result<T, E>,
        mut fold: impl FnMut(Acc, T) -> Return + 'a,
    ) -> impl FnMut(Acc, Item) -> Return + 'a {
        move |acc, item| match f(item) {
            Ok(x) => fold(acc, x),
            Err(_) => try { acc },
        }
    }

    /// Runs a fold method combining two fold functions.
    pub(self) fn fold<Item, T, E, Acc>(
        mut f: impl FnMut(Item) -> Result<T, E>,
        mut fold: impl FnMut(Acc, T) -> Acc,
    ) -> impl FnMut(Acc, Item) -> Acc {
        move |acc, item| match f(item) {
            Ok(x) => fold(acc, x),
            Err(_) => acc,
        }
    }
}

impl<I: std::fmt::Debug, F> std::fmt::Debug for TryFilterMap<I, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TryFilterMap")
            .field("iter", &self.iter)
            .finish_non_exhaustive()
    }
}

impl<T, E, I, F> Iterator for TryFilterMap<I, F>
where
    I: Iterator,
    F: FnMut(I::Item) -> Result<T, E>,
{
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> { self.iter.try_find_map(&mut self.f) }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) { (0, self.iter.size_hint().1) }

    #[inline]
    fn try_fold<Acc, Fold, R>(&mut self, init: Acc, f: Fold) -> R
    where
        Self: Sized,
        Fold: FnMut(Acc, Self::Item) -> R,
        R: std::ops::Try<Output = Acc>,
    {
        self.iter.try_fold(init, Self::try_fold(&mut self.f, f))
    }

    #[inline]
    fn fold<Acc, Fold>(mut self, init: Acc, f: Fold) -> Acc
    where
        Self: Sized,
        Fold: FnMut(Acc, Self::Item) -> Acc,
    {
        self.iter.fold(init, Self::fold(&mut self.f, f))
    }
}

impl<T, E, I, F> DoubleEndedIterator for TryFilterMap<I, F>
where
    I: DoubleEndedIterator,
    F: FnMut(I::Item) -> Result<T, E>,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        #[inline]
        fn find<Item, T, E, F>(f: &mut F) -> impl FnMut((), Item) -> ControlFlow<T> + '_
        where
            F: FnMut(Item) -> Result<T, E>,
        {
            move |(), x| f(x).map_or_else(|_| ControlFlow::Continue(()), ControlFlow::Break)
        }

        self.iter.try_rfold((), find(&mut self.f)).break_value()
    }

    #[inline]
    fn try_rfold<Acc, Fold, Return>(&mut self, init: Acc, f: Fold) -> Return
    where
        Self: Sized,
        Fold: FnMut(Acc, Self::Item) -> Return,
        Return: std::ops::Try<Output = Acc>,
    {
        self.iter.try_rfold(init, Self::try_fold(&mut self.f, f))
    }

    #[inline]
    fn rfold<Acc, Fold>(mut self, init: Acc, f: Fold) -> Acc
    where
        Self: Sized,
        Fold: FnMut(Acc, Self::Item) -> Acc,
    {
        self.iter.fold(init, Self::fold(&mut self.f, f))
    }
}

impl<T, E, I, F> FusedIterator for TryFilterMap<I, F>
where
    I: FusedIterator,
    F: FnMut(I::Item) -> Result<T, E>,
{
}
