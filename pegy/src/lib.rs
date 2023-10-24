extern crate alloc;

mod decoder;
mod source;
mod parse;
mod error;
pub mod util;

#[cfg(feature = "futures")]
pub use futures;

pub use pegy_derive::Parse;

pub use source::{
    Character,
    Source,
    StrSource,
    IntoSource,
    AsyncStrBufReader
};
pub use parse::Parse;
pub use error::{
    Error,
    Span
};

pub type Result<T> = core::result::Result<T, Error>;

pub async fn parse<T:Parse, S: IntoSource>(src: S) -> Result<T::Output>{
    let mut src = src.into();
    T::parse(&mut src).await
}

#[cfg(feature = "futures")]
pub fn parse_blocking<T:Parse, S: IntoSource>(src: S) -> Result<T::Output>{
    futures::executor::block_on(parse::<T, S>(src))
}