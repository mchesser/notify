//! Cross-platform file system notification library
//!
//! # Installation
//!
//! ```toml
//! [dependencies]
//! notify = "5.0.0"
//! ```
//!
//! ## Serde
//!
//! Events are serialisable via [serde] if the `serde` feature is enabled:
//!
//! ```toml
//! notify = { version = "5.0.0-pre.0", features = ["serde"] }
//! ```
//!
//! [serde]: https://serde.rs
//!
//! # Examples
//!
//! ```
//! use crossbeam_channel::unbounded;
//! use notify::{Watcher, RecommendedWatcher, RecursiveMode, Result};
//!
//! fn main() -> Result<()> {
//!     let (tx, rx) = unbounded();
//!
//!     let mut watcher: RecommendedWatcher = Watcher::new_immediate(tx)?;
//!     watcher.watch(".", RecursiveMode::Recursive)?;
//!
//!     loop {
//! #       break;
//!         match rx.recv() {
//!            Ok(event) => println!("event: {:?}", event),
//!            Err(e) => println!("watch error: {:?}", e),
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## With precise events
//!
//! By default, Notify emits non-descript events containing only the affected path and some
//! metadata. To get richer details about _what_ the events are about, you need to enable
//! [`Config::PreciseEvents`](config/enum.Config.html#variant.PreciseEvents). The full event
//! classification is described in the [`event`](event/index.html`) module documentation.
//!
//! ```
//! # use crossbeam_channel::unbounded;
//! # use notify::{Watcher, RecommendedWatcher, Result, watcher};
//! # use std::time::Duration;
//! #
//! # fn main() -> Result<()> {
//! # let (tx, rx) = unbounded();
//! # let mut watcher: RecommendedWatcher = Watcher::new_immediate(tx)?;
//! #
//! use notify::Config;
//! watcher.configure(Config::PreciseEvents(true))?;
//! # Ok(())
//! # }
//! ```
//!
//! ## With different configurations
//!
//! It is possible to create several watchers with different configurations or implementations that
//! all send to the same channel. This can accommodate advanced behaviour or work around limits.
//!
//! ```
//! # use crossbeam_channel::unbounded;
//! # use notify::{Watcher, RecommendedWatcher, RecursiveMode, Result};
//! #
//! # fn main() -> Result<()> {
//! #     let (tx, rx) = unbounded();
//! #
//!       let mut watcher1: RecommendedWatcher = Watcher::new_immediate(tx.clone())?;
//!       let mut watcher2: RecommendedWatcher = Watcher::new_immediate(tx)?;
//! #
//! #     watcher1.watch(".", RecursiveMode::Recursive)?;
//! #     watcher2.watch(".", RecursiveMode::Recursive)?;
//! #
//!       loop {
//! #         break;
//!           match rx.recv() {
//!              Ok(event) => println!("event: {:?}", event),
//!              Err(e) => println!("watch error: {:?}", e),
//!           }
//!       }
//! #
//! #     Ok(())
//! # }
//! ```

#![deny(missing_docs)]

pub use config::{Config, RecursiveMode};
pub use error::{Error, ErrorKind, Result};
pub use event::{Event, EventKind};
pub use raw_event::{op, Op, RawEvent};
use crossbeam_channel::Sender;
use std::convert::AsRef;
use std::path::Path;

#[cfg(target_os = "macos")]
pub use crate::fsevent::FsEventWatcher;
#[cfg(target_os = "linux")]
pub use crate::inotify::INotifyWatcher;
pub use null::NullWatcher;
pub use poll::PollWatcher;
#[cfg(target_os = "windows")]
pub use windows::ReadDirectoryChangesWatcher;

#[cfg(target_os = "macos")]
pub mod fsevent;
#[cfg(target_os = "linux")]
pub mod inotify;
#[cfg(target_os = "windows")]
pub mod windows;

pub mod event;
pub mod null;
pub mod poll;

mod config;
mod debounce;
mod error;
mod raw_event;

/// Type that can deliver file activity notifications
///
/// Watcher is implemented per platform using the best implementation available on that platform.
/// In addition to such event driven implementations, a polling implementation is also provided
/// that should work on any platform.
pub trait Watcher: Sized {
    /// Create a new watcher in _immediate_ mode.
    ///
    /// Events will be sent using the provided `tx` immediately after they occur.
    fn new_immediate(tx: Sender<RawEvent>) -> Result<Self>;

    /// Begin watching a new path.
    ///
    /// If the `path` is a directory, `recursive_mode` will be evaluated. If `recursive_mode` is
    /// `RecursiveMode::Recursive` events will be delivered for all files in that tree. Otherwise
    /// only the directory and its immediate children will be watched.
    ///
    /// If the `path` is a file, `recursive_mode` will be ignored and events will be delivered only
    /// for the file.
    ///
    /// On some platforms, if the `path` is renamed or removed while being watched, behaviour may
    /// be unexpected. See discussions in [#165] and [#166]. If less surprising behaviour is wanted
    /// one may non-recursively watch the _parent_ directory as well and manage related events.
    ///
    /// [#165]: https://github.com/passcod/notify/issues/165
    /// [#166]: https://github.com/passcod/notify/issues/166
    fn watch<P: AsRef<Path>>(&mut self, path: P, recursive_mode: RecursiveMode) -> Result<()>;

    /// Stop watching a path.
    ///
    /// # Errors
    ///
    /// Returns an error in the case that `path` has not been watched or if removing the watch
    /// fails.
    fn unwatch<P: AsRef<Path>>(&mut self, path: P) -> Result<()>;

    /// Configure the watcher at runtime.
    ///
    /// See the [`Config`](config/enum.Config.html) enum for all configuration options.
    ///
    /// # Returns
    ///
    /// - `Ok(true)` on success.
    /// - `Ok(false)` if the watcher does not support or implement the option.
    /// - `Err(notify::Error)` on failure.
    fn configure(&mut self, _option: Config) -> Result<bool> {
        Ok(false)
    }
}

/// The recommended `Watcher` implementation for the current platform
#[cfg(target_os = "linux")]
pub type RecommendedWatcher = INotifyWatcher;
/// The recommended `Watcher` implementation for the current platform
#[cfg(target_os = "macos")]
pub type RecommendedWatcher = FsEventWatcher;
/// The recommended `Watcher` implementation for the current platform
#[cfg(target_os = "windows")]
pub type RecommendedWatcher = ReadDirectoryChangesWatcher;
/// The recommended `Watcher` implementation for the current platform
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub type RecommendedWatcher = PollWatcher;

/// Convenience method for creating the `RecommendedWatcher` for the current platform in
/// _immediate_ mode.
///
/// See [`Watcher::new_immediate`](trait.Watcher.html#tymethod.new_immediate).
pub fn immediate_watcher(tx: Sender<RawEvent>) -> Result<RecommendedWatcher> {
    Watcher::new_immediate(tx)
}
