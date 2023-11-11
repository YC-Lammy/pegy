use crate::{error::Error, Source};

#[allow(async_fn_in_trait)]
pub trait Parse {
    type Output: Default;
    /// function `parse` should not consume any character on failure
    async fn parse<S: Source>(src: &mut S) -> Result<Self::Output, Error>;
}

impl Parse for f64 {
    type Output = f64;

    async fn parse<S: crate::Source>(src: &mut S) -> Result<Self::Output, crate::Error> {
        let start = src.current_position();

        match crate::float::parse(src).await {
            Some(v) => Ok(v),
            None => {
                let end = src.current_position();
                src.set_position(start);
                Err(crate::Error::new(
                    crate::Span(start, end),
                    "error parsing float",
                ))
            }
        }
    }
}

impl Parse for f32 {
    type Output = f32;
    async fn parse<S: Source>(src: &mut S) -> Result<Self::Output, Error> {
        let start = src.current_position();

        match crate::float::parse(src).await {
            Some(v) => Ok(v),
            None => {
                let end = src.current_position();
                src.set_position(start);
                Err(crate::Error::new(
                    crate::Span(start, end),
                    "error parsing float",
                ))
            }
        }
    }
}

impl<T: Parse> Parse for Option<T> {
    type Output = Option<T::Output>;

    #[inline]
    async fn parse<S: Source>(src: &mut S) -> Result<Self::Output, Error> {
        if let Ok(v) = T::parse(src).await {
            return Ok(Some(v));
        }
        return Ok(None);
    }
}

impl Parse for () {
    type Output = ();
    async fn parse<S: Source>(_src: &mut S) -> Result<Self::Output, Error> {
        return Ok(());
    }
}
