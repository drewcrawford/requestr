/*! Drew's high-level HTTP client library for Rust

This library can be compared to [reqwest](https://github.com/seanmonstar/reqwest/blob/master/Cargo.toml), but rather
than remaking the whole world (TLS, HTTP, etc.) in Rust, it wraps some high-level OS API and talks to that instead.  Free for
noncommercial or "small commercial" use.

Advantages:
* Smaller binaries, faster compiles
* Don't have to update dependencies, recompile etc. to resolve security issues or get HTTP3
* Access platform-specific features with a platform-neutral API

Disadvantages:
* Not all features are supported on all platforms
* Some chance behavior changes on newer OS
* Not as popular

Currently supported:
* macOS - uses `NSURLSession` as backend
* windows - uses `HTTPClient` as backend

*/
use std::fmt::{Formatter, Debug};

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::request::Request;

#[cfg(target_os = "windows")]
pub use self::windows::request::Request;

#[cfg(target_os = "windows")]
#[doc(hidden)]
pub use wchar::wchz as __wchz;

#[cfg(target_os = "macos")]
pub use macos::response::{Response,Downloaded};



#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    InvalidURL(String),
    #[cfg(target_os = "windows")]
    PlatformError(::windows::Error),
    PcoreError(pcore::error::Error),
    StatusCode(u16),
}
#[cfg(target_os = "windows")]
impl From<::windows::Error> for Error {
    fn from(e: ::windows::Error) -> Self {
        Error::PlatformError(e)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}",self))
    }
}
impl std::error::Error for Error {}


