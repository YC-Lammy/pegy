use core::marker::PhantomData;

use alloc::{boxed::Box, vec::Vec};

use crate::{Error, Parse, Span};

#[derive(Debug, Default, PartialEq, Eq, Hash)]
pub struct EOF;

impl Parse for EOF {
    type Output = ();
    async fn parse<S: crate::Source>(src: &mut S) -> Result<Self::Output, Error> {
        if src.peek().await.is_some() {
            let pos = src.current_position();
            return Err(Error::new(Span::new(pos, pos), "expected EOF"));
        }
        return Ok(());
    }
}

#[derive(Debug, Default, PartialEq, Eq, Hash)]
pub struct SOF;

impl Parse for SOF {
    type Output = ();
    async fn parse<S: crate::Source>(src: &mut S) -> Result<Self::Output, Error> {
        if src.current_position() != 0 {
            let pos = src.current_position();
            return Err(Error::new(Span::new(pos, pos), "expected SOF"));
        }
        return Ok(());
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Recursive<T: Parse>(pub Box<T::Output>);

impl<T: Parse> Parse for Recursive<T> {
    type Output = T::Output;
    async fn parse<S: crate::Source>(src: &mut S) -> Result<Self::Output, crate::Error> {
        let f = Box::pin(T::parse(src));
        return f.await;
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Boxed<T: Parse>(PhantomData<T>);

impl<T: Parse> Parse for Boxed<T> {
    type Output = Box<T::Output>;
    async fn parse<S: crate::Source>(src: &mut S) -> Result<Self::Output, Error> {
        let value = T::parse(src).await?;
        Ok(Box::new(value))
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Repeat<
    T: Parse,
    const MIN: usize = 0,
    const MAX: usize = 18446744073709551615,
    const SEP: u32 = 4294967295,
>(pub Vec<T::Output>);

impl<T: Parse, const MIN: usize, const MAX: usize, const SEP: u32> Parse
    for Repeat<T, MIN, MAX, SEP>
{
    type Output = Vec<T::Output>;
    async fn parse<S: crate::Source>(src: &mut S) -> Result<Self::Output, crate::Error> {
        let seperator = char::from_u32(SEP);

        let mut v = Vec::new();

        if MAX == 0 {
            return Ok(v);
        }

        let start = src.current_position();

        while let Ok(value) = T::parse(src).await {
            v.push(value);

            if v.len() == MAX {
                break;
            }

            if let Some(ch) = seperator {
                if !src.match_char(ch).await {
                    break;
                }
            }
        }

        if v.len() < MIN {
            let end = src.current_position();
            return Err(Error::new(
                Span::new(start, end),
                "expected minimal number of repeats",
            ));
        }

        return Ok(v);
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct RepeatQuiet<
    T: Parse,
    const MIN: usize = 0,
    const MAX: usize = 18446744073709551615,
    const SEP: u32 = 4294967295,
>(PhantomData<T>);

impl<T: Parse, const MIN: usize, const MAX: usize, const SEP: u32> Parse
    for RepeatQuiet<T, MIN, MAX, SEP>
{
    type Output = ();
    async fn parse<S: crate::Source>(src: &mut S) -> Result<Self::Output, crate::Error> {
        let seperator = char::from_u32(SEP);

        if MAX == 0 {
            return Ok(());
        }

        let mut i = 0;

        let start = src.current_position();

        while let Ok(_) = T::parse(src).await {
            i += 1;
            if i == MAX {
                break;
            }

            if let Some(ch) = seperator {
                if !src.match_char(ch).await {
                    break;
                }
            }
        }

        if i < MIN {
            let end = src.current_position();
            return Err(Error::new(
                Span::new(start, end),
                "expected minimal number of repeats",
            ));
        }

        return Ok(());
    }
}

pub struct AND<A: Parse, B: Parse>(A::Output, B::Output);

impl<A: Parse, B: Parse> Default for AND<A, B> {
    fn default() -> Self {
        Self(A::Output::default(), B::Output::default())
    }
}

impl<A: Parse, B: Parse> Parse for AND<A, B> {
    type Output = Self;
    async fn parse<S: crate::Source>(src: &mut S) -> Result<Self::Output, Error> {
        let a = A::parse(src).await?;

        let b = B::parse(src).await?;
        return Ok(Self(a, b));
    }
}

#[derive(Debug)]
pub enum OR<A: Parse, B: Parse> {
    A(A::Output),
    B(B::Output),
}

impl<A: Parse, B: Parse> Default for OR<A, B> {
    fn default() -> Self {
        Self::A(A::Output::default())
    }
}

impl<A: Parse, B: Parse> Parse for OR<A, B> {
    type Output = Self;
    async fn parse<S: crate::Source>(src: &mut S) -> Result<Self::Output, Error> {
        if let Ok(a) = A::parse(src).await {
            return Ok(Self::A(a));
        }

        let b = B::parse(src).await?;
        return Ok(Self::B(b));
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ANY;

impl Parse for ANY {
    type Output = char;
    async fn parse<S: crate::Source>(src: &mut S) -> Result<Self::Output, Error> {
        if let Some(ch) = src.peek().await {
            let pos = src.current_position() + ch.length;
            src.set_position(pos);
            return Ok(ch.ch);
        }

        let pos = src.current_position();

        return Err(Error::new(Span::new(pos, pos), "expected character"));
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WHITESPACE;

impl Parse for WHITESPACE {
    type Output = char;
    async fn parse<S: crate::Source>(src: &mut S) -> Result<Self::Output, Error> {
        if let Some(ch) = src.peek().await {
            if ch.ch.is_whitespace() {
                let pos = src.current_position() + ch.length;
                src.set_position(pos);
                return Ok(ch.ch);
            }
        }

        let pos = src.current_position();

        return Err(Error::new(Span::new(pos, pos), "expected whitespace"));
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ALPHABETIC;

impl Parse for ALPHABETIC {
    type Output = char;
    async fn parse<S: crate::Source>(src: &mut S) -> Result<Self::Output, Error> {
        if let Some(ch) = src.peek().await {
            if ch.ch.is_alphabetic() {
                let pos = src.current_position() + ch.length;
                src.set_position(pos);
                return Ok(ch.ch);
            }
        }

        let pos = src.current_position();

        return Err(Error::new(
            Span::new(pos, pos),
            "expected alphabetic character",
        ));
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ALPHANUMERIC;

impl Parse for ALPHANUMERIC {
    type Output = char;
    async fn parse<S: crate::Source>(src: &mut S) -> Result<Self::Output, Error> {
        if let Some(ch) = src.peek().await {
            if ch.ch.is_alphanumeric() {
                let pos = src.current_position() + ch.length;
                src.set_position(pos);
                return Ok(ch.ch);
            }
        }

        let pos = src.current_position();

        return Err(Error::new(
            Span::new(pos, pos),
            "expected alphanumeric character",
        ));
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DIGIT<const RADIX: u8 = 16>;

impl<const RADIX: u8> Parse for DIGIT<RADIX> {
    type Output = char;
    async fn parse<S: crate::Source>(src: &mut S) -> Result<Self::Output, Error> {
        if let Some(ch) = src.peek().await {
            if ch.ch.is_digit(RADIX as _) {
                let pos = src.current_position() + ch.length;
                src.set_position(pos);
                return Ok(ch.ch);
            }
        }

        let pos = src.current_position();

        return Err(Error::new(Span::new(pos, pos), "expected digit"));
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CONTROL;

impl Parse for CONTROL {
    type Output = char;
    async fn parse<S: crate::Source>(src: &mut S) -> Result<Self::Output, Error> {
        if let Some(ch) = src.peek().await {
            if ch.ch.is_control() {
                let pos = src.current_position() + ch.length;
                src.set_position(pos);
                return Ok(ch.ch);
            }
        }

        let pos = src.current_position();

        return Err(Error::new(
            Span::new(pos, pos),
            "expected control character",
        ));
    }
}

#[cfg(feature = "unicode")]
#[allow(non_camel_case_types)]
pub struct UNICODE_ID_START;

#[cfg(feature = "unicode")]
impl Parse for UNICODE_ID_START {
    type Output = char;
    async fn parse<S: crate::Source>(src: &mut S) -> Result<Self::Output, Error> {
        if let Some(ch) = src.peek().await {
            if unicode_ident::is_xid_start(ch.ch) {
                src.set_position(src.current_position() + ch.length);
                return Ok(ch.ch);
            }
        }

        let pos = src.current_position();

        return Err(Error::new(
            Span::new(pos, pos),
            "error parsing unicode_id_start",
        ));
    }
}

#[cfg(feature = "unicode")]
#[allow(non_camel_case_types)]
pub struct UNICODE_ID_CONTINUE;

#[cfg(feature = "unicode")]
impl Parse for UNICODE_ID_CONTINUE {
    type Output = char;
    async fn parse<S: crate::Source>(src: &mut S) -> Result<Self::Output, Error> {
        if let Some(ch) = src.peek().await {
            if unicode_ident::is_xid_continue(ch.ch) {
                src.set_position(src.current_position() + ch.length);
                return Ok(ch.ch);
            }
        }

        let pos = src.current_position();

        return Err(Error::new(
            Span::new(pos, pos),
            "error parsing unicode_id_start",
        ));
    }
}
