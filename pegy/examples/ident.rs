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

pub fn main() {
    let re: pegy::Result<Ident> = pegy::parse_blocking::<Ident, _>("myIdent");
    assert!(re.is_ok());
}
