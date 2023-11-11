use std::pin::Pin;
use std::task::Poll;

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

struct StreamAsyncRead<S: futures::Stream<Item = Result<B, reqwest::Error>> + Unpin, B: AsRef<[u8]>>
{
    buffer: Vec<u8>,
    stream: S,
}

impl<S: futures::Stream<Item = Result<B, reqwest::Error>> + Unpin, B: AsRef<[u8]>>
    futures::AsyncRead for StreamAsyncRead<S, B>
{
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        use futures::Stream;

        if !self.buffer.is_empty() {
            let buffer_len = self.buffer.len();
            if buffer_len > buf.len() {
                // copy from buffer
                buf.copy_from_slice(&self.buffer[0..buf.len()]);
                // copy the remain bytes to start
                self.buffer.copy_within(buf.len().., 0);
                // resize buffer
                self.buffer.resize(buffer_len - buf.len(), 0);

                // return copied length
                return Poll::Ready(Ok(buf.len()));
            };

            // copy from buffer
            (&mut buf[0..self.buffer.len()]).copy_from_slice(&self.buffer);
            // clear buffer
            self.buffer.clear();

            return Poll::Ready(Ok(buffer_len));
        }

        let next = Stream::poll_next(Pin::new(&mut self.stream), cx);

        match next {
            Poll::Ready(r) => match r {
                Some(Ok(bytes)) => {
                    let mut bytes = bytes.as_ref();

                    if bytes.len() > buf.len() {
                        self.buffer.extend_from_slice(&bytes[buf.len()..]);
                        bytes = &bytes[..buf.len()];
                    };

                    for (i, b) in bytes.iter().enumerate() {
                        buf[i] = *b;
                    }

                    return Poll::Ready(Ok(bytes.len()));
                }
                Some(Err(e)) => return Poll::Ready(Err(std::io::Error::other(e))),
                None => {
                    return Poll::Ready(Err(std::io::Error::from(
                        std::io::ErrorKind::UnexpectedEof,
                    )))
                }
            },
            Poll::Pending => return Poll::Pending,
        }
    }
}

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

        let src = pegy::AsyncStrSource::new(StreamAsyncRead{
            buffer: Vec::new(),
            stream: request.bytes_stream()
        });

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
