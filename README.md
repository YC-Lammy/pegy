# Pegy - derive based Parser in Rust

## Features
- derive based parser generation
- async api for parsing
- parse input from str, async readers or custom input types.
- AST generation on the run
- meaningful error messages for errors in grammar definition
- supports no_std

## MSRV
The current MSRV supported is 1.75

## Example
```rust
use pegy::Parse;

#[derive(Debug, Default, Parse)]
#[grammar($item0:['a'-'z''A'-'Z''0'-'9'])]
pub struct AlphaDigit(char);

#[derive(Debug, Default, Parse)]
#[grammar($item0:['0'-'9'])]
pub struct Digit(char);

#[derive(Debug, Default, Parse)]
#[grammar(!Digit $item0:AlphaDigit+)]
pub struct Ident(Vec<AlphaDigit>);

pub fn main(){
    let re: pegy::Result<Ident> = pegy::parse_blocking::<Ident, _>("myIdent");
    assert!(re.is_ok());
}
```

## Comparison with similar traits
| crate | action code | integration | input type | streaming input |
| ------| ------------| ------------| ---------- | ----------------|
| pegy  | in grammar  | proc macro(derive) | `&str`, `AsyncRead`, custom | Yes
| peg   | in grammar  | proc macro(block) | `&str`, `&[T]`, custom | No |
| pest  | external | proc macro(file) | `&str` | No |

## Expression Reference
### Term
- `"some string"` - string literal: matches a str slice. returns `&'static str`.
- `'c'` - character literal: matches a character. returns `char`.
- `Ident` - rule: matches a Parse rule. It must be a valid type and imlplements `pegy::Parse`. returns `Ident` type.
- `['a'-'z''A'-'Z''$']` - character class: matches a range of characters. returns `char`.

### Quantifier
- `?` - optional: matches zero or one term. returns `Option<T>`
- `*` - repeat: matches zero or more terms. returns `Vec<T>`
- `+` - repeat atleast: matches one or more terms. returns `Vec<T>`.
- `{min, max}` - repeat range: matches at least `min` and at most `max` number of terms. returns `Vec<T>`

### Special
- `$ident:term` - field binding: bind the result of the term to the field `ident` of result. returns `()`.
- `( alternatives )` - group: matches the terms and returns a `Span`.
- `terms | terms | terms` - alternatives: trys to match the first terms, if failed, matches the second one and so on until a match is found. returns a `Span`.
- `!term` - negative lookahead: matches the term without consuming any characters.
- `_ term` - quiet: matches the term and returns `()`.