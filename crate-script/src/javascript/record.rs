use std::sync::{Arc, Mutex};

use common::v1::types::RunId;

use crate::Result;

// put this in Arc, pass everywhere in qjs?
pub struct RunContextInner {
    replay: Mutex<Replay>,
}

pub type RunContext = Arc<RunContextInner>;

/// for durable execution, record and replay side effects
pub struct Replay {
    /// completed effects
    pub journal: Vec<Effect>,

    /// the current step we are on
    pub cursor: usize,

    /// effects that need to be flushed to the database
    pub pending: Vec<Effect>,
}

impl Replay {
    pub fn new(journal: Vec<Effect>) -> Self {
        Self {
            journal,
            cursor: 0,
            pending: vec![],
        }
    }

    /// get the next the next response in the effect log
    ///
    /// errors if the next request has changed since last run
    pub fn step<E: EffectType>(&mut self, request: &E) -> Result<Option<E::Response>> {
        todo!()
    }

    pub fn record<E: EffectType>(&mut self, request: E, response: E::Response) -> Result<()> {
        todo!()
    }

    // alternative:
    // fn enter(request)
    // fn exit(request)

    // fn drain(self) get all effects to write to db
}

pub struct Effect {
    // pub run_id: RunId,
    // pub started_at: Time,
    // pub ended_at: Option<Time>,
    pub data: EffectData,
}

pub enum EffectData {
    /// an http request
    Fetch(FetchEffect),
    // random
    // time
    // etc...
}

pub struct FetchEffect {
    // http fetch
}

#[derive(Clone)]
pub struct FetchResponse {
    // http fetch
}

impl FetchEffect {
    pub fn new() -> Self {
        todo!()
    }
}

pub trait EffectType {
    type Response;

    /// the response to this effect, or None if hasn't been completed yet
    fn response(&self) -> Option<Self::Response>;
}

impl EffectType for FetchEffect {
    type Response = FetchResponse;

    fn response(&self) -> Option<Self::Response> {
        todo!()
    }
}

struct NetworkManager {
    run_context: RunContext,
}

impl NetworkManager {
    async fn fetch(&self, url: String) -> Result<FetchResponse> {
        let mut replay = self.run_context.replay.lock().unwrap();

        let eff = FetchEffect::new();
        if let Some(res) = replay.step(&eff)? {
            return Ok(res);
        };

        let res: FetchResponse = todo!("do actual fetch");
        replay.record(eff, res.clone())?;

        Ok(res)
    }
}

// fn setup_environment(ctx: &Ctx<'_>, context: Arc<Mutex<JsRunContext>>) -> Result<()> {
//     let globals = ctx.globals();

//     // Override Math.random
//     let ctx_clone = ctx.clone();
//     let context_random = context.clone();
//     globals.set("Math", {
//         let math: rquickjs::Object = globals.get("Math")?;
//         math.set("random", rquickjs::Function::new(ctx_clone, move || {
//             let mut ctx_lock = context_random.lock().unwrap();
//             if let Some(EffectResult::Random(val)) = ctx_lock.get_next_effect("random") {
//                 val
//             } else {
//                 let val = rand::random::<f64>();
//                 ctx_lock.record_effect("random", EffectResult::Random(val));
//                 val
//             }
//         })?)?;
//         math
//     })?;

//     // Repeat similar logic for Date.now()
//     Ok(())
// }

pub enum ExecutionEvent {
    /// an effect was done
    JournalAppend {
        run_id: RunId,
        step_index: usize,
        effect: Effect,
    },
}
