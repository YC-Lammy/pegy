use core::ops::RangeInclusive;

use alloc::vec::Vec;

const CONT_MASK: u8 = 0x3F;

/// Returns the initial codepoint accumulator for the first byte.
/// The first byte is special, only want bottom 5 bits for width 2, 4 bits
/// for width 3, and 3 bits for width 4.
#[inline]
const fn utf8_first_byte(byte: u8, width: u32) -> u32 {
    (byte & (0x7F >> width)) as u32
}

/// Returns the value of `ch` updated with continuation byte `byte`.
#[inline]
const fn utf8_acc_cont_byte(ch: u32, byte: u8) -> u32 {
    (ch << 6) | (byte & CONT_MASK) as u32
}

/// Reads the next code point out of a byte iterator (assuming a
/// UTF-8-like encoding).
///
/// # Safety
///
/// `bytes` must produce a valid UTF-8-like (UTF-8 or WTF-8) string
#[inline]
pub unsafe fn next_code_point<'a, I: Iterator<Item = &'a u8>>(
    bytes: &mut I,
) -> Option<(u32, usize)> {
    let mut i = 1;

    // Decode UTF-8
    let x = *bytes.next()?;
    if x < 128 {
        return Some((x as u32, 1));
    }

    // Multibyte case follows
    // Decode from a byte combination out of: [[[x y] z] w]
    // NOTE: Performance is sensitive to the exact formulation here
    let init = utf8_first_byte(x, 2);
    // SAFETY: `bytes` produces an UTF-8-like string,
    // so the iterator must produce a value here.
    let y = unsafe { *bytes.next().unwrap_unchecked() };
    i += 1;

    let mut ch = utf8_acc_cont_byte(init, y);
    if x >= 0xE0 {
        // [[x y z] w] case
        // 5th bit in 0xE0 .. 0xEF is always clear, so `init` is still valid
        // SAFETY: `bytes` produces an UTF-8-like string,
        // so the iterator must produce a value here.
        let z = unsafe { *bytes.next().unwrap_unchecked() };
        i += 1;

        let y_z = utf8_acc_cont_byte((y & CONT_MASK) as u32, z);
        ch = init << 12 | y_z;
        if x >= 0xF0 {
            // [x y z w] case
            // use only the lower 3 bits of `init`
            // SAFETY: `bytes` produces an UTF-8-like string,
            // so the iterator must produce a value here.
            let w = unsafe { *bytes.next().unwrap_unchecked() };
            i += 1;

            ch = (init & 7) << 18 | utf8_acc_cont_byte(y_z, w);
        }
    }

    Some((ch, i))
}

pub struct Character {
    pub ch: char,
    pub length: usize,
}

#[allow(async_fn_in_trait)]
pub trait Source {
    fn current_position(&self) -> usize;
    fn set_position(&mut self, pos: usize);
    async fn peek(&mut self) -> Option<Character>;
    async fn match_str(&mut self, string: &str) -> bool;
    async fn match_char(&mut self, ch: char) -> bool {
        if let Some(c) = self.peek().await {
            if c.ch == ch {
                self.set_position(self.current_position() + c.length);
                return true;
            }
        }
        return false;
    }
    async fn match_char_range(&mut self, r: RangeInclusive<char>) -> Option<char> {
        if let Some(c) = self.peek().await {
            if r.contains(&c.ch) {
                self.set_position(self.current_position() + c.length);
                return Some(c.ch);
            }
        }
        return None;
    }
}

pub trait IntoSource {
    type Source: Source;
    fn into(self) -> Self::Source;
}

pub struct StrSource<'a> {
    s: &'a str,
    pos: usize,
}

impl<'a> IntoSource for StrSource<'a> {
    type Source = Self;
    fn into(self) -> Self::Source {
        self
    }
}

impl<'a, T: AsRef<str>> IntoSource for &'a T {
    type Source = StrSource<'a>;
    fn into(self) -> Self::Source {
        StrSource::new(self.as_ref())
    }
}

impl<'a> IntoSource for &'a str {
    type Source = StrSource<'a>;
    fn into(self) -> Self::Source {
        StrSource::new(self)
    }
}

impl<'a> StrSource<'a> {
    pub const fn new(s: &'a str) -> Self {
        Self { s: s, pos: 0 }
    }
}

impl<'a> Source for StrSource<'a> {
    #[inline]
    fn current_position(&self) -> usize {
        self.pos
    }
    #[inline]
    fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }
    #[inline]
    async fn peek(&mut self) -> Option<Character> {
        if self.pos == self.s.len() {
            return None;
        }

        unsafe {
            let b = &self.s.as_bytes()[self.pos..];
            let mut iter = b.iter();
            match next_code_point(&mut iter) {
                Some(c) => Some(Character {
                    ch: char::from_u32_unchecked(c.0),
                    length: c.1,
                }),
                None => None,
            }
        }
    }
    #[inline]
    async fn match_str(&mut self, string: &str) -> bool {
        if string.is_empty() {
            return true;
        }
        if self.pos == self.s.len() {
            return false;
        }

        if self.s.len() - self.pos < string.len() {
            return false;
        }

        if (&self.s.as_bytes()[self.pos..self.pos + string.len()]) == string.as_bytes() {
            self.pos += string.len();
            return true;
        }

        return false;
    }
}

#[cfg(feature = "futures")]
pub struct AsyncStrSource<T: futures::AsyncRead + Unpin> {
    reader: T,
    pub buffer: Vec<u8>,
    pos: usize,
    is_eof: bool,
}

impl<T: futures::AsyncRead + Unpin> IntoSource for AsyncStrSource<T> {
    type Source = Self;
    fn into(self) -> Self::Source {
        self
    }
}

#[cfg(feature = "futures")]
impl<T: futures::AsyncRead + Unpin> AsyncStrSource<T> {
    pub fn new(reader: T) -> Self {
        Self {
            reader,
            buffer: Vec::new(),
            pos: 0,
            is_eof: false,
        }
    }

    /// Reads the next code point out of a byte iterator (assuming a
    /// UTF-8-like encoding).
    ///
    /// # Safety
    ///
    /// `bytes` must produce a valid UTF-8-like (UTF-8 or WTF-8) string
    #[inline]
    async fn next_code_point(&mut self) -> Option<(u32, usize)> {
        let mut i = 1;

        // Decode UTF-8
        let x = self.next_u8(0).await?;
        if x < 128 {
            return Some((x as u32, 1));
        }

        // Multibyte case follows
        // Decode from a byte combination out of: [[[x y] z] w]
        // NOTE: Performance is sensitive to the exact formulation here
        let init = utf8_first_byte(x, 2);
        // SAFETY: `bytes` produces an UTF-8-like string,
        // so the iterator must produce a value here.
        let y = self.next_u8(1).await?;
        i += 1;

        let mut ch = utf8_acc_cont_byte(init, y);
        if x >= 0xE0 {
            // [[x y z] w] case
            // 5th bit in 0xE0 .. 0xEF is always clear, so `init` is still valid
            // SAFETY: `bytes` produces an UTF-8-like string,
            // so the iterator must produce a value here.
            let z = self.next_u8(2).await?;
            i += 1;

            let y_z = utf8_acc_cont_byte((y & CONT_MASK) as u32, z);
            ch = init << 12 | y_z;
            if x >= 0xF0 {
                // [x y z w] case
                // use only the lower 3 bits of `init`
                // SAFETY: `bytes` produces an UTF-8-like string,
                // so the iterator must produce a value here.
                let w = self.next_u8(3).await?;
                i += 1;

                ch = (init & 7) << 18 | utf8_acc_cont_byte(y_z, w);
            }
        }

        Some((ch, i))
    }

    #[inline]
    async fn next_u8(&mut self, offset: usize) -> Option<u8> {
        let pos = self.pos + offset;

        if self.buffer.len() == pos && self.is_eof {
            return None;
        }

        if let Some(b) = self.buffer.get(pos) {
            return Some(*b);
        } else {
            loop {
                if let Some(_) = self.read_buf().await {
                    if let Some(b) = self.buffer.get(pos) {
                        return Some(*b);
                    }
                } else {
                    return None;
                };
            }
        }
    }

    #[allow(invalid_value)]
    #[inline]
    async fn read_buf(&mut self) -> Option<usize> {
        use futures::AsyncReadExt;

        if !self.is_eof {
            let mut buf: [u8; 128] = unsafe { core::mem::MaybeUninit::uninit().assume_init() };

            match self.reader.read(&mut buf).await {
                Ok(l) => {
                    self.buffer.extend_from_slice(&buf[..l]);
                    return Some(l);
                }
                Err(_) => {
                    self.is_eof = true;
                }
            };
        }
        return None;
    }
}

#[cfg(feature = "futures")]
impl<T: futures::AsyncRead + Unpin> Source for AsyncStrSource<T> {
    fn current_position(&self) -> usize {
        self.pos
    }
    fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }

    async fn peek(&mut self) -> Option<Character> {
        if self.buffer.len() == self.pos && self.is_eof {
            return None;
        }

        if let Some((c, l)) = self.next_code_point().await {
            return Some(Character {
                ch: unsafe { char::from_u32_unchecked(c) },
                length: l,
            });
        } else {
            return None;
        }
    }

    async fn match_str(&mut self, string: &str) -> bool {
        if string.len() == 0 {
            return true;
        }

        while (self.buffer.len() - self.pos) < string.len() {
            if self.is_eof {
                return false;
            }

            self.read_buf().await;
        }

        if (&self.buffer[self.pos..self.pos + string.len()]) == string.as_bytes() {
            self.pos += string.len();
            return true;
        }

        return false;
    }
}
