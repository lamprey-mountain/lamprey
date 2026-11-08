use rquickjs::{Ctx, Result as JsResult};

const SOURCE: &str = include_str!("abort.js");

/// install the abort controller globals
pub fn setup(ctx: &Ctx<'_>) -> JsResult<()> {
    ctx.eval::<(), _>(SOURCE)
}

#[cfg(test)]
mod tests {
    use rquickjs::{Context, Promise, Runtime};

    use super::setup;

    fn run(source: &str) -> String {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            setup(&ctx).unwrap();
            ctx.eval::<Promise, _>(source)
                .unwrap()
                .finish::<String>()
                .unwrap()
        })
    }

    #[test]
    fn abort_controller() {
        let result = run(r#"
                (async () => {
                    const controller = new AbortController();
                    const signal = controller.signal;
                    let events = 0;
                    let once = 0;
                    signal.addEventListener("abort", () => events++);
                    signal.addEventListener("abort", () => once++, { once: true });

                    controller.abort();
                    controller.abort();

                    let thrown = "";
                    try {
                        signal.throwIfAborted();
                    } catch (error) {
                        thrown = error.name;
                    }

                    return [
                        signal.aborted,
                        signal.reason.name,
                        events,
                        once,
                        thrown,
                    ].join(",");
                })()
            "#);

        assert_eq!(result, "true,AbortError,1,1,AbortError");
    }

    #[test]
    fn static_abort_and_any() {
        let result = run(r#"
                (async () => {
                    const aborted = AbortSignal.abort("stop");
                    const controller = new AbortController();
                    const any = AbortSignal.any([controller.signal]);
                    controller.abort("done");
                    return [
                        aborted.aborted,
                        aborted.reason,
                        any.aborted,
                        any.reason,
                    ].join(",");
                })()
            "#);

        assert_eq!(result, "true,stop,true,done");
    }
}
