extern crate alloc;

mod error;
mod float;
mod integer;
mod parse;
pub mod pratt;
mod source;
pub mod util;

#[cfg(feature = "futures")]
pub use futures;

pub use pegy_derive::Parse;

pub use error::{Error, Span};
pub use parse::Parse;
pub use source::{AsyncStrSource, Character, IntoSource, Source, StrSource};

pub type Result<T> = core::result::Result<T, Error>;

pub async fn parse<T: Parse, S: IntoSource>(src: S) -> Result<T::Output> {
    let mut src = src.into();
    T::parse(&mut src).await
}

#[cfg(feature = "futures")]
pub fn parse_blocking<T: Parse, S: IntoSource>(src: S) -> Result<T::Output> {
    futures::executor::block_on(parse::<T, S>(src))
}
