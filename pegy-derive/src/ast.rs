
use std::marker::PhantomData;

use pegy::util::{
    ANY,
    Recursive,
    Repeat,
    WHITESPACE
};


#[derive(Debug, Default, PartialEq, Eq, pegy::Parse)]
#[grammar($alternatives:Repeat<Alternative, 0, {usize::MAX}, {'|' as u32}>)]
pub struct Alternatives{
    pub preparse: Vec<SpecialTerm>,
    pub alternatives: Vec<Alternative>
}

#[derive(Debug, Default, PartialEq, Eq, pegy::Parse)]
#[grammar($terms:Repeat<SpecialTerm>)]
pub struct Alternative{
    pub terms: Vec<SpecialTerm>
}

#[derive(Debug, PartialEq, Eq, pegy::Parse)]
pub enum SpecialTerm{
    #[grammar(WHITESPACE* '$' $item0:Ident ':' $item1: Term $item2:Quantifier)]
    Binding(String, Term, Quantifier),
    #[grammar(WHITESPACE* '!' $item0:Term $item1:Quantifier)]
    NegativeLookahead(Term, Quantifier),
    #[grammar(WHITESPACE* '_' $item0:Term $item1:Quantifier)]
    Quiet(Term, Quantifier),
    #[grammar($item0: Term $item1:Quantifier)]
    Term(Term, Quantifier)
}

impl Default for SpecialTerm{
    fn default() -> Self {
        Self::Term(Term::String(String::new()), Quantifier::None)
    }
}

#[derive(Debug, PartialEq, Eq, pegy::Parse)]
pub enum Term{
    #[grammar(WHITESPACE* $item0:StringLit WHITESPACE*)]
    String(String),
    #[grammar(WHITESPACE* '\'' $item0:ANY '\'' WHITESPACE*)]
    Character(char),
    #[grammar(WHITESPACE* $item0:StringCapture<Type> WHITESPACE*)]
    Rule(String),
    #[grammar(WHITESPACE* '[' $negative:'^'? ']' WHITESPACE*)]
    CharacterClass{
        negative: Option<char>,
        ranges: Vec<(char, Option<char>)>,
    },
    #[grammar(WHITESPACE* '(' $item0:Recursive<Alternatives> ')' WHITESPACE*)]
    Group(Alternatives),
}

impl Default for Term{
    fn default() -> Self {
        Term::String(String::new())
    }
}

#[derive(Debug, PartialEq, Eq, pegy::Parse)]
pub enum Quantifier{
    #[grammar('?')]
    Optional,
    #[grammar('+')]
    RepeatAtleastOnce,
    #[grammar("**" $seperator:Term)]
    RepeatSeperate{
        seperator: Box<Term>
    },
    #[grammar('*')]
    RepeatUnlimited,
    #[grammar('{' $min:usize ','? '}')]
    RepeatMin{
        min: usize,
    },
    #[grammar('{' $min:usize ',' $max:usize '}')]
    Repeat{
        min: usize,
        max: usize,
    },
    #[grammar("")]
    None
}

impl Default for Quantifier{
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Default, pegy::Parse)]
#[grammar(WHITESPACE* $item0:("::"? "<" (Recursive<Type> ("," Recursive<Type>)*)? ">" | ['a'-'z''A'-'Z''0'-'9''_']+  ("::" ("<" Recursive<Type> ("," Recursive<Type>)* ">" | ['a'-'z''A'-'Z''0'-'9''_']+ ))* ("<" (Recursive<Type> ("," Recursive<Type> )*)? ">")? ) WHITESPACE*)]
struct Type(pegy::Span);

#[derive(Debug, Default)]
struct StringLit;

impl pegy::Parse for StringLit{
    type Output = String;
    async fn parse<S: pegy::Source>(src: &mut S) -> Result<Self::Output, pegy::Error> {
        let mut buf = String::new();
        let start = src.current_position();

        if !src.match_char('"').await{
            return Err(pegy::Error::new(pegy::Span::new(start, start), "error parsing string"))
        }

        buf.push('"');

        loop{
            if src.match_char('"').await{
                buf.push('"');
                break;
            }
            if src.match_str("\\\"").await{
                buf.push_str("\\\"");
            };

            if let Some(c) = src.match_char_range('\0'..=char::MAX).await{
                buf.push(c);
            }
        };

        match syn::parse_str::<syn::LitStr>(&buf){
            Ok(v) => Ok(v.value()),
            Err(e) => {
                let end = src.current_position();
                src.set_position(start);

                Err(pegy::Error::new(pegy::Span::new(start, end), e.to_string()))
            }
        }
    }
}

#[derive(Debug, Default)]
struct Ident;

impl pegy::Parse for Ident{
    type Output = String;
    async fn parse<S: pegy::Source>(src: &mut S) -> Result<Self::Output, pegy::Error> {
        let mut buf = String::new();

        if let Some(ch) = src.peek().await{
            if unicode_id_start::is_id_start(ch.ch){
                buf.push(ch.ch);
                src.set_position(src.current_position() + ch.length);
            } else{
                let pos = src.current_position();
                return Err(pegy::Error::new(pegy::Span::new(pos, pos), "error parsing ident"))
            }
        } else{
            let pos = src.current_position();
            return Err(pegy::Error::new(pegy::Span::new(pos, pos), "error parsing ident"))
        };

        while let Some(ch) = src.peek().await{
            if unicode_id_start::is_id_continue(ch.ch){
                buf.push(ch.ch);
                src.set_position(src.current_position() + ch.length);
            } else{
                break;
            }
        };

        return Ok(buf)
    }
}

#[derive(Debug, Default)]
struct StringCapture<T:pegy::Parse>(PhantomData<T>);

impl<T:pegy::Parse> pegy::Parse for StringCapture<T>{
    type Output = String;
    async fn parse<S: pegy::Source>(src: &mut S) -> Result<Self::Output, pegy::Error> {
        let start = src.current_position();
        let _t = T::parse(src).await?;

        let end = src.current_position();
        src.set_position(start);

        let mut buf = String::with_capacity(end - start);

        while src.current_position() < end{
            if let Some(c) = src.peek().await{
                buf.push(c.ch);
                src.set_position(src.current_position() + c.length);
            }
        };

        return Ok(buf)
    }
}