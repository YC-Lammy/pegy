use crate::Source;

#[allow(invalid_value)]
#[inline]
pub async fn parse<F: fast_float::FastFloat, S: Source>(src: &mut S) -> Option<F> {
    let mut stack_buffer: [u8; 1024] = unsafe { core::mem::MaybeUninit::uninit().assume_init() };
    let mut stack_pos: usize = 0;

    let mut heap_buffer = Vec::new();

    let mut is_neg = false;
    let mut dot = false;

    if src.match_char('-').await {
        stack_buffer[0] = '-' as u8;
        stack_pos += 1;
        is_neg = true;
    } else {
        src.match_char('+').await;
    };

    if let Some(ch) = src.match_char_range('0'..='9').await {
        if ch == '0' {
            // skip zeros
            while src.match_char('0').await {}

            stack_buffer[stack_pos] = '0' as u8;
            stack_pos += 1;
        } else {
            stack_buffer[stack_pos] = ch as u8;
            stack_pos += 1;
        };

        while let Some(ch) = src.match_char_range('0'..='9').await {
            if stack_pos == 1024 {
                heap_buffer.push(ch as u8);
            } else {
                stack_buffer[stack_pos] = ch as u8;
                stack_pos += 1;

                if stack_pos == 1024 {
                    heap_buffer = stack_buffer.to_vec();
                }
            }
        }
    } else if src.match_char('.').await {
        dot = true;

        if stack_pos == 1024 {
            heap_buffer.push('.' as u8);
        } else {
            stack_buffer[stack_pos] = '.' as u8;
            stack_pos += 1;

            if stack_pos == 1024 {
                heap_buffer = stack_buffer.to_vec();
            }
        }
    } else {
        if src.match_str("infinity").await
            || src.match_str("inf").await
            || src.match_str("INFINITY").await
            || src.match_str("INF").await
        {
            if is_neg {
                return Some(F::NEG_INFINITY);
            }
            return Some(F::INFINITY);
        }

        if src.match_str("nan").await || src.match_str("NaN").await || src.match_str("NAN").await {
            if is_neg {
                return Some(F::NEG_NAN);
            }
            return Some(F::NAN);
        }
        return None;
    };

    if !dot {
        if src.match_char('.').await {
            dot = true;

            if stack_pos == 1024 {
                heap_buffer.push('.' as u8);
            } else {
                stack_buffer[stack_pos] = '.' as u8;
                stack_pos += 1;

                if stack_pos == 1024 {
                    heap_buffer = stack_buffer.to_vec();
                }
            }
        }
    };

    if dot {
        while let Some(ch) = src.match_char_range('0'..='9').await {
            if stack_pos == 1024 {
                heap_buffer.push(ch as u8);
            } else {
                stack_buffer[stack_pos] = ch as u8;
                stack_pos += 1;

                if stack_pos == 1024 {
                    heap_buffer = stack_buffer.to_vec();
                }
            }
        }
    };

    let e_pos = src.current_position();
    let mut e_len = 0;

    if src.match_char('e').await || src.match_char('E').await {
        if stack_pos == 1024 {
            heap_buffer.push('e' as u8);
        } else {
            stack_buffer[stack_pos] = 'e' as u8;
            stack_pos += 1;

            if stack_pos == 1024 {
                heap_buffer = stack_buffer.to_vec();
            }
        };

        e_len += 1;

        if src.match_char('-').await {
            if stack_pos == 1024 {
                heap_buffer.push('-' as u8);
            } else {
                stack_buffer[stack_pos] = '-' as u8;
                stack_pos += 1;

                if stack_pos == 1024 {
                    heap_buffer = stack_buffer.to_vec();
                }
            };

            e_len += 1;
        } else {
            src.match_char('+').await;
        };

        if let Some(ch) = src.match_char_range('0'..='9').await {
            if ch == '0' {
                // skip zero
                while src.match_char('0').await {}
            }

            if stack_pos == 1024 {
                heap_buffer.push(ch as u8);
            } else {
                stack_buffer[stack_pos] = ch as u8;
                stack_pos += 1;

                if stack_pos == 1024 {
                    heap_buffer = stack_buffer.to_vec();
                }
            };

            e_len += 1;

            while let Some(ch) = src.match_char_range('0'..='9').await {
                if stack_pos == 1024 {
                    heap_buffer.push(ch as u8);
                } else {
                    stack_buffer[stack_pos] = ch as u8;
                    stack_pos += 1;

                    if stack_pos == 1024 {
                        heap_buffer = stack_buffer.to_vec();
                    }
                };

                e_len += 1;
            }
        } else {
            // no an exponent
            src.set_position(e_pos);

            if stack_pos == 1024 {
                unsafe { heap_buffer.set_len(heap_buffer.len() - e_len) };
            } else {
                stack_pos -= e_len;
            }
        }
    }

    if stack_pos == 1024 {
        fast_float::parse::<F, &[u8]>(&heap_buffer).ok()
    } else {
        fast_float::parse::<F, &[u8]>(&stack_buffer[0..stack_pos]).ok()
    }
}

/*
use super::{binary::compute_float, float::Float};

#[inline]
async fn parse_scientific<S: Source>(src: &mut S) -> Option<i64> {
    let start = src.current_position();
    let mut exp: i64;

    if !src.match_char('e').await {
        if !src.match_char('E').await {
            return None;
        }
    }

    let mut is_neg = false;

    if src.match_char('-').await {
        is_neg = true;
    } else if src.match_char('+').await {
        is_neg = false;
    };

    match src.match_char_range('0'..='9').await {
        Some(c) => {
            let digit = c as i64 - '0' as i64;
            exp = digit;
        }
        None => {
            src.set_position(start);
            return None;
        }
    };

    while let Some(ch) = src.match_char_range('0'..='9').await {
        if exp < i64::MAX{
            exp = exp.saturating_mul(10).saturating_add(ch as i64 - '0' as i64);
        }
    }

    if is_neg {
        return Some(-exp);
    }
    return Some(exp);
}

#[inline]
async fn parse_number_slow<F:Float, S: Source>(
    start: usize,
    src: &mut S,
    is_neg: bool,
    mantissa: u64,
    digits: u32,
    decimal_place: u32,
) -> Option<F> {
    let mut dot = false;
    let mut digits = digits;
    let mut mantissa: u128 = mantissa as u128;

    if decimal_place == 0 {
        while let Some(ch) = src.match_char_range('0'..='9').await {
            digits += 1;

            let digit = ch as u32 - '0' as u32;
            mantissa = mantissa.wrapping_mul(10).wrapping_add(digit as u128);

            if digits == 38 {
                return parse_number_very_slow(start, src, is_neg).await;
            }
        }

        dot = src.match_char('.').await;
    }

    let mut decimal_place = decimal_place as i32;

    if decimal_place != 0 || dot {
        while let Some(ch) = src.match_char_range('0'..='9').await {
            decimal_place += 1;
            digits += 1;

            let digit = ch as u32 - '0' as u32;
            mantissa = mantissa.wrapping_mul(10).wrapping_add(digit as u128);

            if digits == 38 {
                return parse_number_very_slow(start, src, is_neg)
                    .await;
            }
        }
    }

    let mut exponent = -decimal_place as i64;

    if let Some(exp) = parse_scientific(src).await {
        exponent = exponent.saturating_add(exp);
    }

    while mantissa.leading_zeros() < 64 {
        mantissa = mantissa / 10;
        exponent = exponent.saturating_add(1);
    }

    let mut am = compute_float::<F>(exponent, mantissa as u64);

    if am != compute_float::<F>(exponent, (mantissa as u64) + 1){
        src.set_position(start);
        am = super::simple::parse_long_mantissa::<F, S>(src).await
    }

    let mut word = am.mantissa;
    word |= (am.power2 as u64) << 52;
    if is_neg {
        word |= 1_u64 << 63;
    }

    return Some(F::from_bits(word));
}


async fn parse_number_very_slow<F:Float, S: Source>(
    start: usize,
    src: &mut S,
    is_neg: bool,
) -> Option<F> {
    // restart position
    src.set_position(start);

    let am = super::simple::parse_long_mantissa::<F, S>(src).await;

    let mut word = am.mantissa;
    word |= (am.power2 as u64) << 52;
    if is_neg {
        word |= 1_u64 << 63;
    }

    return Some(F::from_bits(word));
}

pub async fn parse_number<F:Float, S: Source>(src: &mut S) -> Option<F> {
    let start = src.current_position();

    let mut is_neg = false;
    let mut mantissa: u64 = 0;
    let mut digits: u32 = 0;
    let mut decimals: u32 = 0;

    if src.match_char('-').await {
        is_neg = true;
    } else if src.match_char('+').await {
        is_neg = false;
    };

    // skip zeros
    while src.match_char('0').await {}

    while let Some(ch) = src.match_char_range('0'..='9').await {
        let digit = ch as u64 - '0' as u64;
        mantissa = mantissa.wrapping_mul(10).wrapping_add(digit);

        digits += 1;

        // if digits hits 19, slow parse
        if digits == 19 {
            return parse_number_very_slow(start, src, is_neg).await;
        }
    }

    let dot = src.match_char('.').await;

    // it is nan or infinite
    if digits == 0 && !dot {
        // try nan or infintity
        if src.match_str("nan").await || src.match_str("NaN").await {
            if is_neg {
                return Some(F::NEG_NAN);
            }
            return Some(F::NAN);
        }
        if src.match_str("infinity").await
            || src.match_str("INFINITY").await
            || src.match_str("inf").await
            || src.match_str("INF").await
        {
            if is_neg {
                return Some(F::NEG_INFINITY);
            }
            return Some(F::INFINITY);
        }

        // it is neither
        return None;
    };

    if dot {
        while let Some(ch) = src.match_char_range('0'..='9').await {
            let digit = ch as u64 - '0' as u64;
            mantissa = mantissa.wrapping_mul(10).wrapping_add(digit);

            digits += 1;
            decimals += 1;

            if digits == 19 {
                return parse_number_very_slow(start, src, is_neg).await;
            }
        }
    }

    let mut exponent = -(decimals as i64);

    if let Some(exp) = parse_scientific(src).await {
        exponent += exp;
    };

    let am = compute_float::<F>(exponent, mantissa);

    let mut word = am.mantissa;
    word |= (am.power2 as u64) << 52;
    if is_neg {
        word |= 1_u64 << 63;
    }

    return Some(F::from_bits(word));
} */
