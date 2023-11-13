use pegy::util::{
    UNICODE_ID_CONTINUE,
    UNICODE_ID_START
};

#[derive(Debug, Default, pegy::Parse)]
#[grammar($item0:UNICODE_ID_START $item1:UNICODE_ID_CONTINUE*)]
pub struct UnicodeIdent(char, Vec<char>);

impl ToString for UnicodeIdent{
    fn to_string(&self) -> String {
        let mut buf = String::with_capacity(self.1.len() + 1);
        buf.push(self.0);
        for c in &self.1{
            buf.push(*c);
        }
        return buf
    }
}

pub fn main(){
    let name = pegy::parse_blocking::<UnicodeIdent, _>("MyIdent");

    assert!(name.is_ok());

    assert_eq!(name.unwrap().to_string(), "MyIdent")
}