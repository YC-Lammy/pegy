use crate::{Source, error::Error, Span};


#[allow(async_fn_in_trait)]
pub trait Parse{
    type Output: Default;
    /// function `parse` should not consume any character on failure
    async fn parse<S:Source>(src: &mut S) -> Result<Self::Output, Error>;
}

macro_rules! parse_uint {
    ($ty:ty) => {
        impl Parse for $ty{
            type Output = Self;
            async fn parse<S:Source>(src: &mut S) -> Result<Self::Output, Error> {
                let start = src.current_position();
                let mut d = false;
                let mut i:$ty = 0;
        
                while let Some(c) = src.match_char_range('0'..='9').await{
                    d = true;
        
                    let t = (c as u32 - '0' as u32) as $ty;
                    if let Some(n) = i.checked_mul(10).and_then(|n|n.checked_add(t)){
                        i = n;
                    } else{
                        let end = src.current_position();
                        src.set_position(start);
        
                        return Err(Error::new(Span::new(start, end), concat!("overflow while parsing ", stringify!($ty))))
                    }
                }
        
                if !d{
                    src.set_position(start);
                    return Err(Error::new(Span::new(start, start), concat!("expected ", stringify!($ty))))
                }
        
                return Ok(i)
            }
        }
    };
}

macro_rules! parse_int {
    ($ty:ty) => {
        impl Parse for $ty{
            type Output = Self;
            async fn parse<S:Source>(src: &mut S) -> Result<Self::Output, Error> {
                let start = src.current_position();
                let mut d = false;
                let mut i:$ty = 0;

                let is_neg = src.match_char('-').await;
        
                while let Some(c) = src.match_char_range('0'..='9').await{
                    d = true;
        
                    let t = (c as u32 - '0' as u32) as $ty;
                    if let Some(n) = i.checked_mul(10).and_then(|n|n.checked_add(t)){
                        i = n;
                    } else{
                        let end = src.current_position();
                        src.set_position(start);
        
                        return Err(Error::new(Span::new(start, end), concat!("overflow while parsing ", stringify!($ty))))
                    }
                }

                if !d{
                    src.set_position(start);
                    return Err(Error::new(Span::new(start, start), concat!("expected ", stringify!($ty))))
                }

                if is_neg{
                    return Ok(-i)
                }
                return Ok(i)
            }
        }
    };
}

parse_uint!(u8);
parse_uint!(u16);
parse_uint!(u32);
parse_uint!(u64);
parse_uint!(u128);
parse_uint!(usize);

parse_int!(i8);
parse_int!(i16);
parse_int!(i32);
parse_int!(i64);
parse_int!(i128);
parse_int!(isize);

impl Parse for f64{
    type Output = f64;
    async fn parse<S:Source>(src: &mut S) -> Result<Self::Output, Error> {
        let start = src.current_position();
        let mut d = false;
        let mut i:usize = 0;

        let is_neg = src.match_char('-').await;

        while let Some(c) = src.match_char_range('0'..='9').await{
            d = true;

            let t = (c as u32 - '0' as u32) as usize;
            if let Some(n) = i.checked_mul(10).and_then(|n|n.checked_add(t)){
                i = n;
            } else{
                let end = src.current_position();
                src.set_position(start);

                return Err(Error::new(Span::new(start, end), "overflow while parsing f64"))
            }
        }

        if !d{
            src.set_position(start);
            return Err(Error::new(Span::new(start, start), "expected f64"))
        };

        
        if src.match_char('.').await{
            let mut de:f64 = 0.0;

            while let Some(c) = src.match_char_range('0'..='9').await{
                let t = (c as u32 - '0' as u32) as usize;

                de = de * 0.1 + t as f64 * 0.1;
            }

            if is_neg{
                return Ok(-(i as f64 + de))
            }
            return Ok(i as f64 + de);
        }

        if is_neg{
            return Ok(-(i as f64))
        }
        return Ok(i as f64)
    }
}

impl Parse for f32{
    type Output = f32;
    async fn parse<S:Source>(src: &mut S) -> Result<Self::Output, Error> {
        let v = f64::parse(src).await?;
        return Ok(v as f32)
    }
}

impl<T:Parse> Parse for Option<T>{
    type Output = Option<T::Output>;

    #[inline]
    async fn parse<S:Source>(src: &mut S) -> Result<Self::Output, Error> {
        if let Ok(v) = T::parse(src).await{
            return Ok(Some(v))
        }
        return Ok(None)
    }
}