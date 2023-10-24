use pegy::Span;
use pegy::Parse;
use pegy::StrSource;
use pegy::util::{
    Repeat,
    RepeatQuiet, 
    WHITESPACE,
    ANY,
    Recursive,
    DIGIT
};

type __ = RepeatQuiet<WHITESPACE>;

#[derive(Debug, Default, Parse)]
pub enum JsonValue{
    #[grammar(__ $item0: f64 __)]
    Number(f64),
    #[grammar(__ $item0:JsonString __)]
    String(JsonString),
    #[default]
    #[grammar(__ "null" __)]
    Null,
    #[grammar(__ $item0:Object __)]
    Object(Object),
    #[grammar(__ $item0:Array __)]
    Array(Array),
}

type ParseFieldValue = Repeat<FieldValue, 0, {usize::MAX}, {',' as u32}>;

#[derive(Debug, Default, Parse)]
#[grammar("{" $item0:ParseFieldValue "}")]
pub struct Object(Vec<FieldValue>);

#[derive(Debug, Default, Parse)]
#[grammar(__ $item0:JsonString __ ":" $item1:Recursive<JsonValue> )]
pub struct FieldValue(JsonString, JsonValue);

type ParseValues = Repeat<Recursive<JsonValue>, 0, {usize::MAX}, {',' as u32}>;

#[derive(Debug, Default, Parse)]
#[grammar("[" $item0:ParseValues "]")]
pub struct Array(Vec<JsonValue>);

#[derive(Debug, Default, Parse)]
#[grammar("\"" $item0:(RepeatQuiet<StringChar>) "\"")]
pub struct JsonString(Span);

#[derive(Debug, Default, Parse)]
#[grammar(!"\"" ("\\" ("r" | "n" | "t" | "v" |("u" DIGIT<16> DIGIT<16> DIGIT<16> DIGIT<16>)) | ANY))]
pub struct StringChar;

pub fn main(){
    let mut src = StrSource::new("{\"a\":[0.0,9.6], \"hello\":78.4, \"hello\":\"world\",\"null\":null}");
    let re = futures::executor::block_on(JsonValue::parse(&mut src));

    println!("{:#?}", re);
}