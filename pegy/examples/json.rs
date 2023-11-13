use pegy::io::AsyncStreamRead;
use pegy::util::{Recursive, Repeat, RepeatQuiet, ANY, DIGIT, WHITESPACE};
use pegy::Parse;
use pegy::Span;

type __ = RepeatQuiet<WHITESPACE>;

#[derive(Debug, Default, Parse)]
pub enum JsonValue {
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

type ParseFieldValue = Repeat<FieldValue, 0, { usize::MAX }, { ',' as u32 }>;

#[derive(Debug, Default, Parse)]
#[grammar("{" $item0:ParseFieldValue "}")]
pub struct Object(Vec<FieldValue>);

#[derive(Debug, Default, Parse)]
#[grammar(__ $item0:JsonString __ ":" $item1:Recursive<JsonValue> )]
pub struct FieldValue(JsonString, JsonValue);

type ParseValues = Repeat<Recursive<JsonValue>, 0, { usize::MAX }, { ',' as u32 }>;

#[derive(Debug, Default, Parse)]
#[grammar("[" $item0:ParseValues "]")]
pub struct Array(Vec<JsonValue>);

#[derive(Debug, Default, Parse)]
#[grammar("\"" $item0:(RepeatQuiet<StringChar>) "\"")]
pub struct JsonString(Span);

#[derive(Debug, Default, Parse)]
#[grammar(!"\"" ("\\" ("r" | "n" | "t" | "v" | "\"" |("u" DIGIT<16> DIGIT<16> DIGIT<16> DIGIT<16>)) | ANY))]
pub struct StringChar;

pub fn main() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()
        .unwrap();

    let now = std::time::Instant::now();

    rt.block_on(async{
        let request = reqwest::get("https://raw.githubusercontent.com/serde-rs/json-benchmark/8b4046037a166b84575635b3662bbd5f9c9d7508/data/canada.json").await
        .unwrap();

        let src = pegy::AsyncStrSource::new(AsyncStreamRead::new(request.bytes_stream()));

        let _value = pegy::parse::<JsonValue, _>(src).await;
    });

    println!("{}", now.elapsed().as_nanos());

    let now = std::time::Instant::now();

    rt.block_on(async{
        let bytes = reqwest::get("https://raw.githubusercontent.com/serde-rs/json-benchmark/8b4046037a166b84575635b3662bbd5f9c9d7508/data/canada.json").await.unwrap()
        .bytes().await.unwrap();

        let mut bytes = bytes.to_vec();

        let _value = simd_json::to_borrowed_value(&mut bytes);
    });

    println!("{}", now.elapsed().as_nanos());
}
