use rquickjs::{Ctx, Module};

#[rquickjs::module(rename = "html")]
mod inner {
    use html5ever::tokenizer::{
        BufferQueue, TagKind, Token, TokenSinkResult, Tokenizer, TokenizerOpts,
    };
    use rquickjs::{class::Trace, Ctx, Function, JsLifetime, Result, Value};

    use crate::script::types::JsResponse;

    #[derive(Trace, JsLifetime)]
    #[rquickjs::class()]
    pub struct HtmlParser<'js> {
        on_token: Function<'js>,
    }

    #[rquickjs::methods]
    impl<'js> HtmlParser<'js> {
        #[qjs(constructor)]
        pub fn new(on_token: Function<'js>) -> Self {
            Self { on_token }
        }

        pub async fn handle(&self, ctx: Ctx<'js>, mut res: JsResponse) -> Result<()> {
            let mut inner_res = res.inner.take().ok_or(rquickjs::Error::Exception)?;
            let sink = JsTokenSink {
                ctx: ctx.clone(),
                callback: self.on_token.clone(),
            };
            let tokenizer = Tokenizer::new(sink, TokenizerOpts::default());
            let mut queue = BufferQueue::default();

            while let Ok(Some(chunk)) = inner_res.chunk().await {
                let s = String::from_utf8_lossy(&chunk);
                queue.push_back(s.to_string().into());
                let _ = tokenizer.feed(&mut queue);
            }
            tokenizer.end();
            Ok(())
        }
    }

    struct JsTokenSink<'js> {
        ctx: Ctx<'js>,
        callback: Function<'js>,
    }

    impl<'js> html5ever::tokenizer::TokenSink for JsTokenSink<'js> {
        type Handle = ();
        fn process_token(
            &self,
            token: html5ever::tokenizer::Token,
            _line_number: u64,
        ) -> TokenSinkResult<()> {
            // Convert html5ever Token to a JS Object
            // This is a simplified example of the mapping
            let js_val = match token {
                Token::TagToken(tag) => {
                    let obj = rquickjs::Object::new(self.ctx.clone()).unwrap();
                    let _ = obj.set(
                        "type",
                        if tag.kind == TagKind::StartTag {
                            "StartTag"
                        } else {
                            "EndTag"
                        },
                    );
                    let _ = obj.set("name", tag.name.to_string());
                    Value::from_object(obj)
                }
                Token::CharacterTokens(s) => {
                    let obj = rquickjs::Object::new(self.ctx.clone()).unwrap();
                    let _ = obj.set("type", "Text");
                    let _ = obj.set("content", s.to_string());
                    Value::from_object(obj)
                }
                _ => return TokenSinkResult::Continue,
            };

            let _ = self.callback.call::<_, ()>((js_val,));
            TokenSinkResult::Continue
        }
    }
}

pub fn register(ctx: Ctx<'_>) -> rquickjs::Result<()> {
    Module::declare_def::<js_inner, _>(ctx, "lamprey:html")?;
    Ok(())
}
