//! Provides solutions for multithreading for the Doop Discord bot.
#![deny(clippy::expect_used, unsafe_code, clippy::unwrap_used)]
#![warn(clippy::nursery, clippy::todo, clippy::pedantic, missing_docs)]
#![allow(clippy::module_name_repetitions)]

use std::ops::{Deref, DerefMut};
// use std::sync::mpsc::{Receiver, RecvError, RecvTimeoutError, SendError, Sender,
// TryRecvError};
use std::sync::OnceLock;
use std::thread::{Builder, JoinHandle, Thread};
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvError, RecvTimeoutError, SendError, Sender, TryRecvError};

/// A thread handle.
#[repr(transparent)]
#[derive(Debug)]
pub struct Handle<T: Send + 'static> {
    /// The inner join handle.
    inner: JoinHandle<T>,
}

impl<T: Send + 'static> Handle<T> {
    /// Spawns a new thread with the given name and function.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    pub fn spawn<F>(name: impl AsRef<str>, f: F) -> std::io::Result<Self>
    where
        F: (FnOnce() -> T) + Send + 'static,
    {
        // Remove null bytes, avoiding a possible panic.
        let name = name.as_ref().replace('\0', r"\0");
        let inner = Builder::new().name(name).spawn(f)?;

        Ok(Self { inner })
    }

    /// Returns a reference to the underlying thread of this [`Handle<T>`].
    #[inline]
    #[must_use]
    pub fn thread(&self) -> &Thread {
        self.inner.thread()
    }

    /// Waits for the associated thread to finish execution.
    ///
    /// # Panics
    ///
    /// Panics if the thread panicked while joining.
    #[inline]
    #[must_use]
    pub fn join(self) -> T {
        #[allow(clippy::unwrap_used)]
        self.inner.join().unwrap()
    }

    /// Returns whether the thread is finished executing.
    #[inline]
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.inner.is_finished()
    }
}

impl<T: Send + 'static> HandledThread<T> for Handle<T> {
    #[inline]
    fn as_handle(&self) -> &Self {
        self
    }

    #[inline]
    fn as_handle_mut(&mut self) -> &mut Self {
        self
    }

    #[inline]
    fn into_handle(self) -> Self {
        self
    }

    #[inline]
    fn join(self) -> T {
        self.join()
    }
}

/// Represents a thread with an associated join handle.
pub trait HandledThread<T: Send + 'static> {
    /// Returns a reference to the inner thread handle.
    fn as_handle(&self) -> &Handle<T>;

    /// Returns a mutable reference to the inner thread handle.
    fn as_handle_mut(&mut self) -> &mut Handle<T>;

    /// Unwraps and returns the thread handle.
    fn into_handle(self) -> Handle<T>;

    /// Waits for the thread to finish execution.
    ///
    /// # Panics
    ///
    /// Panics if the thread panicked while joining.
    #[must_use]
    fn join(self) -> T;

    /// Creates a new [`AutoJoin<T, R>`] wrapper containing this thread.
    #[inline]
    fn auto(self) -> AutoJoin<Self, T>
    where
        Self: Sized,
    {
        AutoJoin::new(self)
    }

    /// Creates a new [`AutoJoin<T, R>`] wrapper containing this thread.
    ///
    /// This wrapper will contain a closure that is called *before* the thread is joined.
    #[inline]
    fn auto_cleaned(self, f: fn(&mut Self)) -> AutoJoin<Self, T>
    where
        Self: Sized,
    {
        AutoJoin::new_cleaned(self, f)
    }

    /// Creates a new [`AutoJoin<T, R>`] wrapper containing this thread.
    ///
    /// This wrapper will contain a closure that is called *after* the thread is joined.
    #[inline]
    fn auto_handled(self, f: fn(T)) -> AutoJoin<Self, T>
    where
        Self: Sized,
    {
        AutoJoin::new_handled(self, f)
    }

    /// Creates a new [`AutoJoin<T, R>`] wrapper containing this thread.
    ///
    /// This wrapper will contain closures that are called before *and* after the thread is joined.
    #[inline]
    fn auto_managed(self, before: fn(&mut Self), after: fn(T)) -> AutoJoin<Self, T>
    where
        Self: Sized,
    {
        AutoJoin::new_managed(self, before, after)
    }
}

/// A thread that consumes values through a sender.
pub trait SenderThread<S: Send + 'static> {
    /// Returns a cloned sender linked to this thread.
    fn clone_sender(&self) -> Sender<S>;

    /// Attempts to send a value to the thread.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread's receiver channel is closed.
    fn send(&self, value: S) -> Result<(), SendError<S>>;

    /// Attempts to send a stream of values to the thread.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread's receiver channel is closed.
    fn send_all(&self, values: impl IntoIterator<Item = S>) -> Result<(), SendError<S>> {
        values.into_iter().try_for_each(|value| self.send(value))
    }
}

/// A thread that produces values through a receiver.
pub trait ReceiverThread<R: Send + 'static> {
    /// Attempts to receive a value from the thread.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread's sender channel is closed.
    fn recv(&self) -> Result<R, RecvError>;

    /// Attempts to receive a value from the thread, timing out after the given duration is elapsed.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread's sender channel is closed or the timeout
    /// has elapsed.
    fn recv_timeout(&self, timeout: Duration) -> Result<R, RecvTimeoutError>;

    /// Attempts to receive a value from the thread if one is immediately available.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread's sender channel is closed or a value is
    /// not available to be received.
    fn try_recv(&self) -> Result<R, TryRecvError>;

    /// Returns an iterator of receivable values from the thread's sender channel, blocking until at
    /// least one is available.
    fn recv_iter(&self) -> crossbeam_channel::Iter<'_, R>;

    /// Returns an iterator of receivable values from the thread's sender channel.
    fn try_recv_iter(&self) -> crossbeam_channel::TryIter<'_, R>;

    /// Attempts to receive `N` values from the thread.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread's sender channel is closed.
    fn recv_count<const N: usize>(&self) -> Result<[R; N], RecvError> {
        let iter = std::iter::repeat_with(|| self.recv()).take(N);
        let list = iter.collect::<Result<Vec<_>, _>>()?;

        // Safety: the vector will always be `N` elements long and can therefore always be
        // converted into an array of size `N`.
        #[allow(unsafe_code)]
        Ok(unsafe { list.try_into().unwrap_unchecked() })
    }

    /// Attempts to receive `N` values from the thread, timing out after the given duration is
    /// elapsed.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread's sender channel is closed or the timeout
    /// has elapsed.
    fn recv_timeout_count<const N: usize>(
        &self,
        timeout: Duration,
    ) -> Result<[R; N], RecvTimeoutError> {
        let iter = std::iter::repeat_with(|| self.recv_timeout(timeout)).take(N);
        let list = iter.collect::<Result<Vec<_>, _>>()?;

        // Safety: the vector will always be `N` elements long and can therefore always be
        // converted into an array of size `N`.
        #[allow(unsafe_code)]
        Ok(unsafe { list.try_into().unwrap_unchecked() })
    }
}

/// A thread that consumes values through a sender.
#[derive(Debug)]
pub struct Consumer<S, T>
where
    S: Send + 'static,
    T: Send + 'static,
{
    /// The thread's inner handle.
    handle: Handle<T>,
    /// The thread's sender channel.
    sender: Sender<S>,
}

impl<S, T> Consumer<S, T>
where
    S: Send + 'static,
    T: Send + 'static,
{
    /// Spawns a new thread with the given name and function.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    pub fn spawn<F>(name: impl AsRef<str>, f: F) -> std::io::Result<Self>
    where
        F: (FnOnce(Receiver<S>) -> T) + Send + 'static,
    {
        let (sender, receiver) = crossbeam_channel::unbounded();

        Ok(Self { handle: Handle::spawn(name, move || f(receiver))?, sender })
    }
}

impl<S, T> HandledThread<T> for Consumer<S, T>
where
    S: Send + 'static,
    T: Send + 'static,
{
    #[inline]
    fn as_handle(&self) -> &Handle<T> {
        &self.handle
    }

    #[inline]
    fn as_handle_mut(&mut self) -> &mut Handle<T> {
        &mut self.handle
    }

    #[inline]
    fn into_handle(self) -> Handle<T> {
        self.handle
    }

    #[inline]
    fn join(self) -> T {
        self.handle.join()
    }
}

impl<S, T> SenderThread<S> for Consumer<S, T>
where
    S: Send + 'static,
    T: Send + 'static,
{
    #[inline]
    fn clone_sender(&self) -> Sender<S> {
        self.sender.clone()
    }

    #[inline]
    fn send(&self, value: S) -> Result<(), SendError<S>> {
        self.sender.send(value)
    }
}

/// A thread that produces values through a receiver.
#[derive(Debug)]
pub struct Producer<R, T>
where
    R: Send + 'static,
    T: Send + 'static,
{
    /// The thread's inner handle.
    handle: Handle<T>,
    /// The thread's receiver channel.
    receiver: Receiver<R>,
}

impl<R, T> Producer<R, T>
where
    R: Send + 'static,
    T: Send + 'static,
{
    /// Spawns a new thread with the given name and function.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    pub fn spawn<F>(name: impl AsRef<str>, f: F) -> std::io::Result<Self>
    where
        F: (FnOnce(Sender<R>) -> T) + Send + 'static,
    {
        let (sender, receiver) = crossbeam_channel::unbounded();

        Ok(Self { handle: Handle::spawn(name, move || f(sender))?, receiver })
    }
}

impl<R, T> HandledThread<T> for Producer<R, T>
where
    R: Send + 'static,
    T: Send + 'static,
{
    #[inline]
    fn as_handle(&self) -> &Handle<T> {
        &self.handle
    }

    #[inline]
    fn as_handle_mut(&mut self) -> &mut Handle<T> {
        &mut self.handle
    }

    #[inline]
    fn into_handle(self) -> Handle<T> {
        self.handle
    }

    #[inline]
    fn join(self) -> T {
        self.handle.join()
    }
}

impl<R, T> ReceiverThread<R> for Producer<R, T>
where
    R: Send + 'static,
    T: Send + 'static,
{
    #[inline]
    fn recv(&self) -> Result<R, RecvError> {
        self.receiver.recv()
    }

    #[inline]
    fn recv_timeout(&self, timeout: Duration) -> Result<R, RecvTimeoutError> {
        self.receiver.recv_timeout(timeout)
    }

    #[inline]
    fn try_recv(&self) -> Result<R, TryRecvError> {
        self.receiver.try_recv()
    }

    #[inline]
    fn recv_iter(&self) -> crossbeam_channel::Iter<'_, R> {
        self.receiver.iter()
    }

    #[inline]
    fn try_recv_iter(&self) -> crossbeam_channel::TryIter<'_, R> {
        self.receiver.try_iter()
    }
}

/// A thread that consumes and produces values through a sender and receiver.
#[derive(Debug)]
pub struct Exchanger<S, R, T>
where
    S: Send + 'static,
    R: Send + 'static,
    T: Send + 'static,
{
    /// The thread's inner handle.
    handle: Handle<T>,
    /// The thread's sender channel.
    sender: Sender<S>,
    /// The thread's receiver channel.
    receiver: Receiver<R>,
}

impl<S, R, T> Exchanger<S, R, T>
where
    S: Send + 'static,
    R: Send + 'static,
    T: Send + 'static,
{
    /// Spawns a new thread with the given name and function.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    pub fn spawn<F>(name: impl AsRef<str>, f: F) -> std::io::Result<Self>
    where
        F: (FnOnce(Sender<R>, Receiver<S>) -> T) + Send + 'static,
    {
        let (local_sender, thread_receiver) = crossbeam_channel::unbounded();
        let (thread_sender, local_receiver) = crossbeam_channel::unbounded();
        let handle = Handle::spawn(name, move || f(thread_sender, thread_receiver))?;

        Ok(Self { handle, sender: local_sender, receiver: local_receiver })
    }
}

impl<S, R, T> HandledThread<T> for Exchanger<S, R, T>
where
    S: Send + 'static,
    R: Send + 'static,
    T: Send + 'static,
{
    #[inline]
    fn as_handle(&self) -> &Handle<T> {
        &self.handle
    }

    #[inline]
    fn as_handle_mut(&mut self) -> &mut Handle<T> {
        &mut self.handle
    }

    #[inline]
    fn into_handle(self) -> Handle<T> {
        self.handle
    }

    #[inline]
    fn join(self) -> T {
        self.handle.join()
    }
}

impl<S, R, T> SenderThread<S> for Exchanger<S, R, T>
where
    S: Send + 'static,
    R: Send + 'static,
    T: Send + 'static,
{
    #[inline]
    fn clone_sender(&self) -> Sender<S> {
        self.sender.clone()
    }

    #[inline]
    fn send(&self, value: S) -> Result<(), SendError<S>> {
        self.sender.send(value)
    }
}

impl<S, R, T> ReceiverThread<R> for Exchanger<S, R, T>
where
    S: Send + 'static,
    R: Send + 'static,
    T: Send + 'static,
{
    #[inline]
    fn recv(&self) -> Result<R, RecvError> {
        self.receiver.recv()
    }

    #[inline]
    fn recv_timeout(&self, timeout: Duration) -> Result<R, RecvTimeoutError> {
        self.receiver.recv_timeout(timeout)
    }

    #[inline]
    fn try_recv(&self) -> Result<R, TryRecvError> {
        self.receiver.try_recv()
    }

    #[inline]
    fn recv_iter(&self) -> crossbeam_channel::Iter<'_, R> {
        self.receiver.iter()
    }

    #[inline]
    fn try_recv_iter(&self) -> crossbeam_channel::TryIter<'_, R> {
        self.receiver.try_iter()
    }
}

/// Automatically joins and discards the return value of a thread when dropped.
///
/// This wrapper also allows a specified closure to be called before joining or after joining, to
/// allow cleanup or return value handling.
#[derive(Debug)]
pub struct AutoJoin<T, R>
where
    T: HandledThread<R>,
    R: Send + 'static,
{
    /// The inner joinable thread.
    inner: OnceLock<T>,
    /// The closure called before joining.
    before: Option<fn(&mut T)>,
    /// The closure called after joining.
    after: Option<fn(R)>,
}

impl<T, R> AutoJoin<T, R>
where
    T: HandledThread<R>,
    R: Send + 'static,
{
    /// Creates a new [`AutoJoin<T, R>`] struct from the given thread and closures.
    #[allow(unsafe_code)]
    fn new_from_parts(inner: T, before: Option<fn(&mut T)>, after: Option<fn(R)>) -> Self {
        let cell = OnceLock::new();

        // Safety: this cell was just initialized and is guaranteed to be empty.
        unsafe { cell.set(inner).unwrap_unchecked() };

        Self { inner: cell, before, after }
    }

    /// Creates a new [`AutoJoin<T, R>`] wrapper containing the given thread.
    #[inline]
    pub fn new(inner: T) -> Self {
        Self::new_from_parts(inner, None, None)
    }

    /// Creates a new [`AutoJoin<T, R>`] wrapper containing the given thread.
    ///
    /// This wrapper will contain a closure that is called *before* the thread is joined.
    #[inline]
    pub fn new_cleaned(inner: T, f: fn(&mut T)) -> Self {
        Self::new_from_parts(inner, Some(f), None)
    }

    /// Creates a new [`AutoJoin<T, R>`] wrapper containing the given thread.
    ///
    /// This wrapper will contain a closure that is called *after* the thread is joined.
    #[inline]
    pub fn new_handled(inner: T, f: fn(R)) -> Self {
        Self::new_from_parts(inner, None, Some(f))
    }

    /// Creates a new [`AutoJoin<T, R>`] wrapper containing the given thread.
    ///
    /// This wrapper will contain closures that are called before *and* after the thread is joined.
    #[inline]
    pub fn new_managed(inner: T, before: fn(&mut T), after: fn(R)) -> Self {
        Self::new_from_parts(inner, Some(before), Some(after))
    }

    /// Returns whether the inner thread has been joined and dropped.
    pub fn has_dropped(&self) -> bool {
        self.inner.get().is_none()
    }
}

impl<T, R> Deref for AutoJoin<T, R>
where
    T: HandledThread<R>,
    R: Send + 'static,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // this will only panic mid-drop, where nothing should be accessing it anyways.
        #[allow(clippy::expect_used)]
        self.inner.get().expect("the thread has already been joined")
    }
}

impl<T, R> DerefMut for AutoJoin<T, R>
where
    T: HandledThread<R>,
    R: Send + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        // this will only panic mid-drop, where nothing should be accessing it anyways.
        #[allow(clippy::expect_used)]
        self.inner.get_mut().expect("the thread has already been joined")
    }
}

impl<T, R> Drop for AutoJoin<T, R>
where
    T: HandledThread<R>,
    R: Send + 'static,
{
    fn drop(&mut self) {
        // If `self.inner` is uninitialized, assume we've already dropped.
        let Some(mut thread) = self.inner.take() else {
            return;
        };

        // Allow thread cleanup if a closure is specified.
        if let Some(before) = self.before {
            before(&mut thread);
        }

        // Drop the return value if a closure is not specified.
        self.after.unwrap_or(drop)(thread.join());
    }
}
