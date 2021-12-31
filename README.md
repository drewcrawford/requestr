# requestr

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