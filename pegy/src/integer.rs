use core::fmt::{Debug, Display};
use core::ops::{Add, BitAnd, Div, Mul, Shl, Sub};

use crate::{Error, Parse, Source, Span};

pub trait Integer:
    Sized
    + Div<Output = Self>
    + Mul<Output = Self>
    + Add<Output = Self>
    + Sub<Output = Self>
    + Shl<u32, Output = Self>
    + BitAnd<Output = Self>
    + PartialEq
    + Eq
    + PartialOrd
    + Ord
    + Default
    + Clone
    + Copy
    + Debug
    + Display
{
    const MAX_SAFE_DIGITS: usize;
    const MIN: Self;
    const MAX: Self;

    fn from_u8(v: u8) -> Self;
    fn wrapping_mul(self, rhs: Self) -> Self;
    fn wrapping_add(self, rhs: Self) -> Self;
    fn checked_mul(self, rhs: Self) -> Option<Self>;
    fn checked_add(self, rhs: Self) -> Option<Self>;
}

impl<I: Integer> Parse for I {
    type Output = I;
    async fn parse<S: Source>(src: &mut S) -> Result<Self::Output, Error> {
        let start = src.current_position();

        let mut i = I::from_u8(0);
        let mut has_digt = false;
        let mut is_neg = false;

        // parse negative sign
        if I::MIN < I::from_u8(0) {
            if src.match_char('-').await {
                is_neg = true;
            }
        }

        if src.match_char('0').await {
            has_digt = true;

            if src.match_char('x').await {
                let mut digits = 0;
                let max_digits: usize = core::mem::size_of::<I>() * 2;

                // ignore zeros
                if src.match_char('0').await {
                    while src.match_char('0').await {}
                }

                while let Some(c) = src.peek().await {
                    if digits == max_digits {
                        let end = src.current_position();
                        src.set_position(start);
                        return Err(Error::new(
                            Span::new(start, end),
                            "overflow while parsing integer",
                        ));
                    }

                    if let Some(digit) = c.ch.to_digit(16) {
                        digits += 1;

                        src.set_position(src.current_position() + c.length);

                        i = i
                            .wrapping_mul(I::from_u8(16))
                            .wrapping_add(I::from_u8(digit as u8));
                    } else {
                        break;
                    }
                }

                if digits == 0 {
                    let end = src.current_position();
                    src.set_position(start);
                    return Err(Error::new(Span::new(start, end), "error parsing integer"));
                }

                if is_neg {
                    return Ok(I::from_u8(0) - i);
                }

                return Ok(i);
            }

            if src.match_char('b').await {
                let mut digits = 0;
                let max_digits: usize = core::mem::size_of::<I>() * 8;

                // ignore zeros
                if src.match_char('0').await {
                    while src.match_char('0').await {}
                }

                while let Some(c) = src.match_char_range('0'..='1').await {
                    digits += 1;
                    if c == '1' {
                        i = (i << 1) & I::from_u8(1);
                    } else {
                        i = i << 1;
                    }
                }

                if digits == 0 {
                    let end = src.current_position();
                    src.set_position(start);
                    return Err(Error::new(Span::new(start, end), "error parsing integer"));
                }

                if digits > max_digits {
                    let end = src.current_position();
                    src.set_position(start);
                    return Err(Error::new(
                        Span::new(start, end),
                        "overflow while parsing integer",
                    ));
                }

                if is_neg {
                    return Ok(I::from_u8(0) - i);
                }

                return Ok(i);
            }

            // skip zeros
            while src.match_char('0').await {}
        };

        if let Some(c) = src.match_char_range('0'..='9').await {
            has_digt = true;
            i = I::from_u8(c as u8 - '0' as u8);

            for _ in 0..I::MAX_SAFE_DIGITS - 1 {
                if let Some(c) = src.match_char_range('0'..='9').await {
                    let digit = c as u8 - '0' as u8;
                    i = i
                        .wrapping_mul(I::from_u8(10))
                        .wrapping_add(I::from_u8(digit));
                } else {
                    return Ok(i);
                }
            }
        }

        if !has_digt {
            let end = src.current_position();
            src.set_position(start);
            return Err(Error::new(Span::new(start, end), "error parsing integer"));
        }

        while let Some(c) = src.match_char_range('0'..='9').await {
            if let Some(m) = i.checked_mul(I::from_u8(10)) {
                if let Some(n) = m.checked_add(I::from_u8(c as u8 - '0' as u8)) {
                    i = n;
                    continue;
                }
            };

            let end = src.current_position();
            src.set_position(start);
            return Err(Error::new(
                Span::new(start, end),
                "overflow while parsing integer",
            ));
        }

        if is_neg {
            return Ok(I::from_u8(0) - i);
        }

        return Ok(i);
    }
}

macro_rules! impl_int {
    ($ty:ty, $s:tt) => {
        impl Integer for $ty {
            const MAX: Self = <$ty>::MAX;
            const MIN: Self = <$ty>::MIN;
            const MAX_SAFE_DIGITS: usize = $s;

            #[inline]
            fn from_u8(v: u8) -> Self {
                v as _
            }
            #[inline]
            fn wrapping_add(self, rhs: Self) -> Self {
                <$ty>::wrapping_add(self, rhs)
            }
            #[inline]
            fn wrapping_mul(self, rhs: Self) -> Self {
                <$ty>::wrapping_mul(self, rhs)
            }
            #[inline]
            fn checked_add(self, rhs: Self) -> Option<Self> {
                <$ty>::checked_add(self, rhs)
            }
            #[inline]
            fn checked_mul(self, rhs: Self) -> Option<Self> {
                <$ty>::checked_mul(self, rhs)
            }
        }
    };
}

impl_int!(u8, 2);
impl_int!(u16, 4);
impl_int!(u32, 9);
impl_int!(u64, 19);
impl_int!(u128, 38);
#[cfg(target_pointer_width = "64")]
impl_int!(usize, 19);
#[cfg(target_pointer_width = "32")]
impl_int!(usize, 9);

impl_int!(i8, 2);
impl_int!(i16, 4);
impl_int!(i32, 9);
impl_int!(i64, 18);
impl_int!(i128, 38);
#[cfg(target_pointer_width = "64")]
impl_int!(isize, 18);
#[cfg(target_pointer_width = "32")]
impl_int!(isize, 9);
