#[macro_export]
macro_rules! pstr {
    ($expr: literal) => {
        crate::__wchz!(u16,$expr)
    }
}