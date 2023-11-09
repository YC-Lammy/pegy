use alloc::borrow::Cow;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span(pub usize, pub usize);

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self(start, end)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Error {
    pub msg: Cow<'static, str>,
    pub span: Span,
}

impl Default for Error {
    fn default() -> Self {
        Self {
            msg: Cow::Borrowed("unknown"),
            span: Span(0, 0),
        }
    }
}

impl Error {
    pub fn new<S: Into<Cow<'static, str>>>(span: Span, msg: S) -> Error {
        Error {
            msg: msg.into(),
            span: span,
        }
    }
}
