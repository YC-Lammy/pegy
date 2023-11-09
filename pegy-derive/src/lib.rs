use std::str::FromStr;

use proc_macro2::{Punct, Spacing, TokenStream, TokenTree};
use quote::{ToTokens, TokenStreamExt};

mod grammar;

#[proc_macro_derive(Parse, attributes(grammar))]
pub fn parse_derive(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let re = syn::parse::<syn::Item>(stream);

    match re {
        Ok(item) => match item {
            syn::Item::Struct(s) => compile_struct(s).into(),
            syn::Item::Enum(e) => compile_enum(e).into(),
            _ => quote::quote!(compile_error!("Parse macro expected struct or enum.")).into(),
        },
        Err(e) => e.into_compile_error().into(),
    }
}

fn compile_struct(s: syn::ItemStruct) -> proc_macro2::TokenStream {
    let mut declares = TokenStream::new();

    let ident = &s.ident;

    for (index, field) in s.fields.iter().enumerate() {
        let ty = &field.ty;
        if let Some(id) = &field.ident {
            declares.extend(quote::quote!(let mut #id:#ty = ::core::default::Default::default();))
        } else {
            let mut id = "item".to_string();
            id.push_str(itoa::Buffer::new().format(index));

            let id = TokenStream::from_str(&id).unwrap();
            declares.extend(quote::quote!(let mut #id:#ty = ::core::default::Default::default();))
        }
    }

    let mut grammar = TokenStream::new();
    for attr in &s.attrs {
        if attr.path().is_ident("grammar") {
            let re = attr.parse_args_with(grammar::Parser::default());

            match re {
                Ok(g) => {
                    grammar = g;
                    break;
                }
                Err(e) => return e.into_compile_error(),
            }
        }
    }

    if grammar.is_empty() {
        return syn::Error::new(ident.span(), "missing grammar").into_compile_error();
    }

    let mut construction = TokenStream::new();

    match &s.fields {
        syn::Fields::Named(n) => {
            for field in &n.named {
                construction.extend(field.ident.as_ref().to_token_stream());
                construction.append(TokenTree::Punct(Punct::new(',', Spacing::Alone)));
            }

            construction = quote::quote!(Self{#construction});
        }
        syn::Fields::Unnamed(n) => {
            for (index, _field) in n.unnamed.iter().enumerate() {
                let mut id = "item".to_string();
                id.push_str(itoa::Buffer::new().format(index));

                let id = TokenStream::from_str(&id).unwrap();

                construction.extend(id);
                construction.append(TokenTree::Punct(Punct::new(',', Spacing::Alone)));
            }

            construction = quote::quote!(Self(#construction));
        }
        syn::Fields::Unit => {
            construction = quote::quote!(Self);
        }
    }

    return quote::quote! {
        impl ::pegy::Parse for #ident{
            type Output = Self;
            async fn parse<S: ::pegy::Source>(src: &mut S) -> Result<Self::Output, ::pegy::Error>{
                #declares;
                let _start = src.current_position();
                let mut _error = None;

                let re = #grammar;

                match re{
                    Ok(_) => Ok(#construction),
                    Err(e) => {
                        src.set_position(_start);
                        Err(e)
                    }
                }
            }
        }
    };
}

fn compile_enum(e: syn::ItemEnum) -> TokenStream {
    let mut variants = TokenStream::new();

    for varient in &e.variants {
        let mut declares = TokenStream::new();

        let ident = &varient.ident;

        for (index, field) in varient.fields.iter().enumerate() {
            let ty = &field.ty;
            if let Some(id) = &field.ident {
                declares
                    .extend(quote::quote!(let mut #id:#ty = ::core::default::Default::default();))
            } else {
                let mut id = "item".to_string();
                id.push_str(itoa::Buffer::new().format(index));

                let id = TokenStream::from_str(&id).unwrap();
                declares
                    .extend(quote::quote!(let mut #id:#ty = ::core::default::Default::default();))
            }
        }

        let mut grammar = TokenStream::new();
        for attr in &varient.attrs {
            if attr.path().is_ident("grammar") {
                let re = attr.parse_args_with(grammar::Parser::default());

                match re {
                    Ok(g) => {
                        grammar = g;
                        break;
                    }
                    Err(e) => return e.into_compile_error(),
                }
            }
        }

        if grammar.is_empty() {
            return syn::Error::new(ident.span(), "missing grammar").into_compile_error();
        }

        let mut construction = TokenStream::new();

        match &varient.fields {
            syn::Fields::Named(n) => {
                for field in &n.named {
                    construction.extend(field.ident.as_ref().to_token_stream());
                    construction.append(TokenTree::Punct(Punct::new(',', Spacing::Alone)));
                }

                construction = quote::quote!(Self::#ident{#construction});
            }
            syn::Fields::Unnamed(n) => {
                for (index, _field) in n.unnamed.iter().enumerate() {
                    let mut id = "item".to_string();
                    id.push_str(itoa::Buffer::new().format(index));

                    let id = TokenStream::from_str(&id).unwrap();

                    construction.extend(id);
                    construction.append(TokenTree::Punct(Punct::new(',', Spacing::Alone)));
                }
                construction = quote::quote!(Self::#ident (#construction));
            }
            syn::Fields::Unit => {
                construction = quote::quote!(Self::#ident);
            }
        }

        variants.extend(quote::quote! {
            {
                #declares;
                let mut _error = None;

                let re = #grammar;

                match re{
                    Ok(_) => return Ok(#construction),
                    Err(_) => {
                        src.set_position(_start);
                    }
                }
            };
        });
    }

    let enum_id = &e.ident;
    return quote::quote! {
        impl ::pegy::Parse for #enum_id{
            type Output = Self;
            async fn parse<S: ::pegy::Source>(src:&mut S) -> ::pegy::Result<Self::Output>{
                let _start = src.current_position();
                #variants;
                return Err(::pegy::Error::new(::pegy::Span::new(_start, _start), concat!("expected ", stringify!(#enum_id))));
            }
        }
    };
}
