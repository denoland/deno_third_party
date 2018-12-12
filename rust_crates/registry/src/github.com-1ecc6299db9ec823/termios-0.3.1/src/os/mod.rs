//! OS-specific definitions.

#[cfg(target_os = "linux")] pub use self::linux as target;
#[cfg(target_os = "android")] pub use self::android as target;
#[cfg(target_os = "macos")] pub use self::macos as target;
#[cfg(target_os = "freebsd")] pub use self::freebsd as target;
#[cfg(target_os = "openbsd")] pub use self::openbsd as target;
#[cfg(target_os = "dragonfly")] pub use self::dragonfly as target;

#[cfg(target_os = "linux")] pub mod linux;
#[cfg(target_os = "android")] pub mod android;
#[cfg(target_os = "macos")] pub mod macos;
#[cfg(target_os = "freebsd")] pub mod freebsd;
#[cfg(target_os = "openbsd")] pub mod openbsd;
#[cfg(target_os = "dragonfly")] pub mod dragonfly;
