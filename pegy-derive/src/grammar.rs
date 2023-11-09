use std::str::FromStr;

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::{ToTokens, TokenStreamExt};

#[derive(Default)]
pub struct Parser;

impl syn::parse::Parser for Parser {
    type Output = TokenStream;
    fn parse2(mut self, tokens: proc_macro2::TokenStream) -> syn::Result<Self::Output> {
        let tokens = tokens.into_iter().collect::<Vec<TokenTree>>();
        let mut pos = 0;
        let stream = self.parse_alternative(&tokens, &mut pos, false)?;

        return Ok(stream);
    }
}

impl Parser {
    pub fn parse_alternative(
        &mut self,
        tokens: &[TokenTree],
        pos: &mut usize,
        is_silent: bool,
    ) -> syn::Result<TokenStream> {
        if *pos == tokens.len() {
            return Ok(quote::quote!(Ok::<(), ::pegy::Error>(())));
        }

        let mut stream = TokenStream::new();

        while *pos < tokens.len() {
            let terms = self.parse_terms(tokens, pos, is_silent)?;

            if let Some(t) = tokens.get(*pos) {
                *pos += 1;

                match t {
                    TokenTree::Punct(p) => {
                        if p.as_char() != '|' {
                            return Err(syn::Error::new(t.span(), "expected '|' or term."));
                        }
                    }
                    _ => return Err(syn::Error::new(t.span(), "expected '|' or term.")),
                };
            }

            if stream.is_empty() {
                stream.extend(quote::quote! {
                    if match #terms{
                        Ok(_) => true,
                        Err(e) => {
                            _error = Some(e);
                            false
                        }
                    }{
                        let _end = src.current_position();
                        Ok(::pegy::Span::new(_start, _end))
                    }
                });
            } else {
                stream.extend(quote::quote! {
                    else if match {src.set_position(_start); #terms}{
                        Ok(_) => true,
                        Err(e) => {
                            _error = Some(e);
                            false
                        }
                    }{
                        let _end = src.current_position();
                        Ok(::pegy::Span::new(_start, _end))
                    }
                });
            }
        }

        stream.extend(quote::quote! {
            else{
                src.set_position(_start);
                Err(_error.take().unwrap())
            }
        });

        return Ok(quote::quote! {
            {
                let _start = src.current_position();
                #stream
            }
        });
    }

    pub fn parse_terms(
        &mut self,
        tokens: &[TokenTree],
        pos: &mut usize,
        is_silent: bool,
    ) -> syn::Result<TokenStream> {
        let mut terms = TokenStream::new();

        let mut breaking = "'t".to_string();
        breaking.push_str(
            &std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                .to_string(),
        );

        let breaking = TokenStream::from_str(&breaking).unwrap();

        let mut ids = Vec::new();

        while *pos < tokens.len() {
            if let Some(TokenTree::Punct(p)) = tokens.get(*pos) {
                if p.as_char() == '|' {
                    break;
                }
            }
            let term = self.parse_term(tokens, pos, is_silent)?;

            let mut sid = "_item".to_string();
            sid.push_str(itoa::Buffer::new().format(ids.len()));

            let id = TokenStream::from_str(&sid).unwrap();
            ids.push(sid);

            terms.extend(quote::quote! {
                let #id = match #term{
                    Ok(v) => v,
                    Err(e) => break #breaking Err(e)
                };
            });
        }

        let ids = ids.join(",");
        let ids = TokenStream::from_str(&ids).unwrap();

        return Ok(quote::quote! {
            #breaking:{
                #terms
                Ok::<_, ::pegy::Error>((#ids))
            }
        });
    }

    pub fn parse_term(
        &mut self,
        tokens: &[TokenTree],
        pos: &mut usize,
        is_silent: bool,
    ) -> syn::Result<TokenStream> {
        let n = if let Some(n) = tokens.get(*pos) {
            *pos += 1;
            n
        } else {
            return Ok(quote::quote!(Ok::<(), ::pegy::Error>(())));
        };

        let stream;

        match n {
            TokenTree::Group(g) => match g.delimiter() {
                Delimiter::Brace => {
                    return Err(syn::Error::new(g.span(), "unexpected quantifier"));
                }
                Delimiter::Parenthesis | Delimiter::None => {
                    let new_tokens = g.stream().into_iter().collect::<Vec<TokenTree>>();
                    let mut new_pos = 0;

                    let terms = self.parse_alternative(&new_tokens, &mut new_pos, is_silent)?;
                    stream = quote::quote_spanned! { g.span() =>
                        {
                            let _start = src.current_position();
                            match #terms{
                                Ok(_) => {
                                    let _end = src.current_position();
                                    Ok(::pegy::Span::new(_start, _end))
                                }
                                Err(e) => Err(e)
                            }
                        }
                    };
                }
                Delimiter::Bracket => {
                    stream = self.parse_character_class(g.stream())?;
                }
            },
            TokenTree::Ident(i) => {
                let mut id = i.to_token_stream();

                if let Some(TokenTree::Punct(p)) = tokens.get(*pos) {
                    if p.as_char() == '<' {
                        *pos += 1;
                        id.append(TokenTree::Punct(p.clone()));

                        let mut closed = false;

                        while let Some(t) = tokens.get(*pos) {
                            *pos += 1;
                            id.append(t.clone());
                            match t {
                                TokenTree::Punct(p) => {
                                    if p.as_char() == '>' {
                                        closed = true;
                                        break;
                                    }
                                }
                                _ => {}
                            }
                        }

                        if !closed {
                            return Err(syn::Error::new(p.span(), "missing '>' token"));
                        }
                    };
                };

                stream =
                    quote::quote_spanned!(i.span() => <#id as ::pegy::Parse>::parse(src).await);
            }
            TokenTree::Literal(lit) => {
                let l = lit.to_string();

                if l.starts_with('"') {
                    stream = quote::quote_spanned! { lit.span() =>
                        if src.match_str(#lit).await{
                            Ok(#lit)
                        } else{
                            let _pos = src.current_position();
                            Err(::pegy::Error::new(::pegy::Span::new(_pos, _pos), concat!("expected string literal '", #lit, "'")))
                        }
                    };
                } else {
                    stream = quote::quote_spanned! {lit.span() =>
                        if src.match_char(#lit).await{
                            Ok(#lit)
                        } else{
                            let _pos = src.current_position();
                            Err(::pegy::Error::new(::pegy::Span::new(_pos, _pos), concat!("expected character '", #lit, "'")))
                        }
                    };
                }
            }
            TokenTree::Punct(p) => {
                if p.as_char() == '$' {
                    if let Some(TokenTree::Ident(id)) = tokens.get(*pos) {
                        *pos += 1;
                        if let Some(TokenTree::Punct(p)) = tokens.get(*pos) {
                            if p.as_char() == ':' {
                                *pos += 1;
                                let term = self.parse_term(tokens, pos, is_silent)?;

                                // skip the quantifier
                                return Ok(quote::quote! {
                                    match #term{
                                        Ok(v) => {
                                            let _start = src.current_position();
                                            #id = v.into();
                                            let _end = src.current_position();
                                            Ok(::pegy::Span::new(_start, _end))
                                        },
                                        Err(e) => Err(e)
                                    }
                                });
                            } else {
                                return Err(syn::Error::new(p.span(), "expected ':' token."));
                            }
                        } else {
                            return Err(syn::Error::new(p.span(), "expected ':' token."));
                        }
                    } else {
                        return Err(syn::Error::new(
                            p.span(),
                            "expected ident behind binding declaration",
                        ));
                    }
                } else if p.as_char() == '!' {
                    let term = self.parse_term(tokens, pos, is_silent)?;
                    stream = quote::quote_spanned! { p.span() =>
                        {
                            let _start = src.current_position();
                            match #term{
                                Ok(_) => {
                                    let _end = src.current_position();
                                    Err(::pegy::Error::new(::pegy::Span::new(_start, _end), "negative lookahead failed"))
                                }
                                Err(_) => {
                                    let _end = src.current_position();
                                    src.set_position(_start);
                                    Ok(::pegy::Span::new(_start, _end))
                                }
                            }
                        }

                    };
                } else if p.as_char() == '_' {
                    stream = self.parse_term(tokens, pos, true)?;
                } else {
                    return Err(syn::Error::new(p.span(), "unexpected token"));
                }
            }
        };

        return Ok(self.parse_quantifier(tokens, pos, is_silent, stream)?);
    }

    pub fn parse_quantifier(
        &mut self,
        tokens: &[TokenTree],
        pos: &mut usize,
        is_silent: bool,
        term: TokenStream,
    ) -> syn::Result<TokenStream> {
        if let Some(TokenTree::Punct(p)) = tokens.get(*pos) {
            if p.as_char() == '+' {
                *pos += 1;
                if is_silent {
                    return Ok(quote::quote! {
                        {
                            let _start = src.current_position();
                            let mut _i:usize = 0;
                            while let Ok(_value) = #term{
                                _i += 1;
                            };

                            if _i == 0{
                                Err(::pegy::Error::new(::pegy::Span::new(_start, _start), "expected at least one repetition"))
                            } else{
                                Ok(())
                            }
                        }
                    });
                }
                return Ok(quote::quote! {
                    {
                        let _start = src.current_position();
                        let mut _v = Vec::new();
                        while let Ok(_value) = #term{
                            _v.push(_value);
                        };

                        if _v.len() == 0{
                            Err(::pegy::Error::new(::pegy::Span::new(_start, _start), "expected at least one repetition"))
                        } else{
                            Ok(_v)
                        }
                    }
                });
            }

            if p.as_char() == '*' {
                *pos += 1;

                if is_silent {
                    return Ok(quote::quote! {
                        {
                            while let Ok(_value) = #term{};
                            Ok::<(), ::pegy::Error>(())
                        }
                    });
                }
                return Ok(quote::quote! {
                    {
                        let mut _v = Vec::new();
                        while let Ok(_value) = #term{
                            _v.push(_value);
                        };
                        Ok::<_, ::pegy::Error>(_v)
                    }
                });
            }

            if p.as_char() == '?' {
                *pos += 1;

                if is_silent {
                    return Ok(quote::quote! {
                        if let Ok(_v) = #term{
                            Ok::<Option<()>, ::pegy::Error>(Some(()))
                        } else{
                            Ok(None)
                        }
                    });
                }
                return Ok(quote::quote! {
                    if let Ok(v) = #term{
                        Ok::<_, ::pegy::Error>(Some(v))
                    } else{
                        Ok(None)
                    }
                });
            }
        }

        if let Some(TokenTree::Group(g)) = tokens.get(*pos) {
            if g.delimiter() == Delimiter::Brace {
                *pos += 1;

                let mut min = TokenStream::new();
                let mut max = TokenStream::new();

                for t in g.stream().into_iter() {
                    match t {
                        TokenTree::Literal(l) => {
                            if min.is_empty() {
                                min.append(TokenTree::Literal(l.clone()));
                            } else if max.is_empty() {
                                max.append(TokenTree::Literal(l.clone()));
                            }
                        }
                        TokenTree::Punct(p) => {
                            if p.as_char() == ',' {
                                if min.is_empty() {
                                    min.extend(quote::quote!(0));
                                }
                                if !max.is_empty() {
                                    return Err(syn::Error::new(p.span(), "unexpected token"));
                                }
                            } else {
                                return Err(syn::Error::new(p.span(), "unexpected token"));
                            }
                        }
                        _ => return Err(syn::Error::new(t.span(), "unexpected token")),
                    }
                }

                if min.is_empty() {
                    return Err(syn::Error::new(g.span(), "missing range specifier"));
                }

                if max.is_empty() {
                    max = quote::quote!(18446744073709551615);
                }

                if is_silent {
                    return Ok(quote::quote! {
                        {
                            let _start = src.current_position();
                            let mut _i:usize = 0;
                            while let Ok(_value) = #term{
                                _i += 1;

                                if _i >= #max{
                                    break;
                                }
                            };

                            if _i < #min{
                                Err(::pegy::Error::new(::pegy::Span::new(_start, _start), concat!("expected at least ", #min, "repetition")))
                            } else{
                                Ok(())
                            }
                        }
                    });
                }
                return Ok(quote::quote! {
                    {
                        let _start = src.current_position();
                        let mut _v = Vec::new();
                        while let Ok(_value) = #term{
                            _v.push(_value);

                            if _v.len() >= #max{
                                break;
                            }
                        };

                        if _v.len() < #min{
                            Err(::pegy::Error::new(::pegy::Span::new(_start, _start), concat!("expected at least ", #min, "repetition")))
                        } else{
                            Ok(_v)
                        }
                    }
                });
            }
        }

        if is_silent {
            return Ok(quote::quote! {
                match #term{
                    Ok(_) => Ok::<(), ::pegy::Error>(()),
                    Err(e) => Err(e)
                }
            });
        }

        return Ok(term);
    }

    pub fn parse_character_class(&mut self, stream: TokenStream) -> syn::Result<TokenStream> {
        let mut ranges = Vec::new();
        let mut last_char: Option<proc_macro2::Literal> = None;
        let mut is_continue = false;

        for token in stream.into_iter() {
            match token {
                TokenTree::Literal(l) => {
                    if let Some(last) = last_char.take() {
                        if is_continue {
                            ranges.push((last, l))
                        } else {
                            ranges.push((last.clone(), last));
                            last_char = Some(l);
                        }
                    } else {
                        last_char = Some(l);
                    }
                }
                TokenTree::Punct(p) => {
                    if p.as_char() != '-' {
                        return Err(syn::Error::new(p.span(), "expected '-' or character"));
                    }
                    if last_char.is_none() {
                        return Err(syn::Error::new(p.span(), "expected character"));
                    }
                    is_continue = true;
                }
                _ => return Err(syn::Error::new(token.span(), "unexpected token")),
            }
        }

        if ranges.is_empty() {
            return Ok(quote::quote! {{
                let _start = src.current_position();
                Err::<char, ::pegy::Error>(::pegy::Error::new(_start, _start), "failed to match character class")
            }});
        }

        let mut stream = TokenStream::new();

        for (start, end) in ranges {
            if stream.is_empty() {
                stream.extend(quote::quote_spanned! { start.span() =>
                    if let Some(_ch) = src.match_char_range(#start..=#end).await{
                        Ok(_ch)
                    }
                })
            } else {
                stream.extend(quote::quote_spanned! { start.span() =>
                    else if let Some(_ch) = src.match_char_range(#start..=#end).await{
                        Ok(_ch)
                    }
                })
            }
        }

        stream.extend(quote::quote!{
            else{
                let _pos = src.current_position();
                Err(::pegy::Error::new(::pegy::Span::new(_pos, _pos), "failed to match character class"))
            }
        });

        return Ok(stream);
    }
}
