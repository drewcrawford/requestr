/*! Drew's high-level request library

This library can be compared to [reqwest](https://github.com/seanmonstar/reqwest/blob/master/Cargo.toml), but rather
than remaking the whole world (TLS, HTTP, etc.) in Rust, it wraps some high-level OS API and talks to that instead.

Advantages:
* Smaller binaries, faster compiles
* Don't have to update, recompile etc. to resolve security issues or get HTTP3
* Access platform-specific features with a platform-neutral API

Disadvantages:
* Not all features are supported on all platforms
* Some chance behavior changes on newer OS
* Not as popular

Currently supported:
* macOS - uses NSURLSession as backend

*/
use std::fmt::{Formatter, Debug};

mod macos;

pub use macos::request::Request;
pub use macos::response::{Response,Downloaded};

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    InvalidURL(String),
    PlatformError(pcore::error::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}",self))
    }
}
impl std::error::Error for Error {}

impl Error {
    fn with_perror(error: pcore::error::Error) -> Self {
        Self::PlatformError(error)
    }
}

