//! random sketch of what the api would look like from a wasm redex's pov

// all of this is pretty complex if i want to support wasm
// building my own lang might make the api easier, but idk...
// then i'd need to build my own language!

/// types *inside* of a redex
mod effect {
    trait Observability {
        fn log(&self, level: RedexLogLevel, data: &str);
        fn log_with(&self, level: RedexLogLevel, data: &str, attrs: &[(&str, &str)]);
        // s/span/trace/g?
        // id = 0 -> random gen, reuse id = reuse span
        fn span_enter(&self, level: RedexLogLevel, name: &str, id: u64) -> u64;
        fn span_tag(&self, key: &str, value: &str);
        fn span_exit(&self, level: RedexLogLevel);
        fn metric_set(&self, counter: &str, val: u64);
        fn metric_incr(&self, counter: &str, val: u64);
        fn metric_count(&self, counter: &str);
    }

    trait Api {
        fn room_member_get(&self) -> RoomMember;
        fn room_member_update(&self) -> RoomMember;
        fn room_member_kick(&self) -> RoomMember;
        fn invite_resolve(&self) -> Invite;
        fn invite_delete(&self);
        fn invite_room_create(&self) -> Invite;
        fn invite_thread_create(&self) -> Invite;
        fn room_edit(&self) -> Room;
        fn message_create(&self) -> Message;
        // etc...
    }

    // all of this seems way too low level...
    trait Datastore {
        fn store_create(&self, id: &str);
        fn store_delete(&self, id: &str);
        fn store_set_ttl(&self, id: &str, ttl: u64); // in ms
        fn store_poll(&self, store: &str, last_id: u64) -> bool;
        fn store_poll_blocking(&self, store: &str, last_id: u64);
        fn store_poll_next(&self, store: &str, last_id: u64) -> (u64, Vec<u8>); // last_id, key
        fn store_list(&self, store: &str) -> Vec<String>;
        fn object_set(&self, store: &str, key: Vec<u8>, value: Vec<u8>);
        fn object_del(&self, store: &str, key: Vec<u8>);
        fn object_get(&self, store: &str, key: Vec<u8>) -> Option<Vec<u8>>;
        fn object_count(&self, store: &str, key: Vec<u8>, delta: u64);
        fn object_add(&self, store: &str, key: Vec<u8>, delta: u64); // Uint/Int only
        fn object_min(&self, store: &str, key: Vec<u8>, n: u64); // Uint/Int only
        fn object_max(&self, store: &str, key: Vec<u8>, n: u64); // Uint/Int only
        fn object_list_after(
            &self,
            store: &str,
            from: Option<Vec<u8>>,
            to: Option<Vec<u8>>,
            limit: u64,
        );
        fn object_list_before(
            &self,
            store: &str,
            from: Option<Vec<u8>>,
            to: Option<Vec<u8>>,
            limit: u64,
        );
        fn object_list_prefix(
            &self,
            store: &str,
            prefix: Vec<u8>,
            from: Option<Vec<u8>>,
            to: Option<Vec<u8>>,
            limit: u64,
        );
        fn index_create(&self, store: &str, name: &str, map: FnOnce(&[u8]) -> impl Into<Vec<u8>>);
        fn index_delete(&self, store: &str, name: &str);
        fn index_list(&self, store: &str) -> Vec<String>;
        fn transaction_begin(&self, write: bool) -> u64;
        fn transaction_commit(&self, txn: u64);
        fn transaction_rollback(&self, txn: u64);
    }

    // can be used to send stuff between tasks
    trait Queue {
        /// lookup or create by name
        fn queue_ensure(&self, id: &str) -> u64;
        /// create a new anonymous queue
        fn queue_anonymous(&self) -> u64;
        fn queue_delete(&self, id: u64);
        fn queue_is_closed(&self, id: u64) -> bool;
        fn queue_set_max_len(&self, id: u64, len: u64);
        fn queue_set_ttl(&self, id: u64, ms: u64);
        /// messages always are received in the order they're enqueued, otherwise high delay msg wont block low delay msg
        fn queue_set_strict_order(&self, id: u64, strict: bool);
        /// whether this redex should be woken up when an item is ready in this queue
        fn queue_set_wakes(&self, id: u64, wakes: bool);
        fn queue_list(&self, id: u64) -> Vec<String>;
        fn queue_push(&self, id: u64, data: Vec<u8>, delay: Option<u64>);
        fn queue_pull(&self, id: u64, limit: u64) -> Option<(u64, Vec<u8>)>;
        fn queue_pull_blocking(&self, id: u64, limit: u64) -> (u64, Vec<u8>);
        /// commit, pulled item is removed from queue
        fn queue_consume(&self, id: u64, item_id: u64);
        /// rollback, pulled item is added to front of queue (will be repulled next)
        fn queue_abort(&self, id: u64, item_id: u64);
        /// rollback, pulled item is added to back of queue
        fn queue_retry(&self, id: u64, item_id: u64, delay: Option<u64>);
        fn queue_truncate(&self, id: u64, max_len: u64);
        fn queue_length(&self, id: u64) -> u64;
        fn queue_poll(&self, id: u64, block: bool) -> bool;
    }

    trait Timer {
        /// unix time in milliseconds
        fn timer_get(&self) -> u64;
        fn timer_schedule(&self, cron: &str, job: &str);
        fn timer_schedule_once(&self, cron: &str, job: &str);
        fn timer_cancel(&self, job: &str);
        fn timer_list(&self) -> Vec<u8>;
    }

    enum AttrTy {
        Uint,
        Int,
        Uuid,
        String,
        // hint to server to store as json (whitespace may not be preserved), useful for read/write in api
        Json,
        // generally use for everything else
        Bytes,
    }

    #[repr(u8)]
    enum HttpState {
        Creating = 0,
        Connecting = 1,
        Uploading = 2,
        Receiving = 3,
        Downloading = 4,
        Finished = 5,
        Errored = 6,
    }

    trait Net {
        // idk about this, it should probably be pretty restricted to prevent abuse
        fn http_create(&self, method: &str, url: &str) -> u64;
        fn http_set_header(&self, id: u64, name: &str, val: &str);
        fn http_set_timeout(&self, id: u64, ms: u64);
        fn http_set_body(&self, id: u64, body: Vec<u8>);
        fn http_set_callback(&self, id: u64, cb: FnOnce() -> ());
        fn http_send(&self, id: u64);
        fn http_poll(&self, id: u64) -> HttpState; // also used for getting the state
        fn http_poll_blocking(&self, id: u64); // until the status changes, returns immediately if Finished or Errored
        fn http_free(&self, id: u64);
        fn http_get_status(&self, id: u64) -> u16;
        fn http_get_headers(&self, id: u64) -> Vec<(String, String)>;
        fn http_get_header(&self, id: u64, key: &str) -> Option<String>;
        fn http_body_read(&self, id: u64, max_size: u64) -> Vec<u8>;
        fn http_body_poll(&self, id: u64, min_size: u64) -> bool;
        fn http_body_poll_blocking(&self, id: u64, min_size: u64);
        fn http_body_set_callback(&self, id: u64, cb: FnOnce(Vec<u8>) -> ()); // prob very dangerous
    }

    trait Flow {
        /// send this outside of this redex and as an event that a bot can handle
        fn cont(&self, name: &str, payload: Vec<u8>) -> !;

        /// send this outside of this redex and as an event that a bot can handle, then return and continue executing
        fn call(&self, name: &str, payload: Vec<u8>) -> Vec<u8>;

        /// panic!
        fn panic(&self, why: &str);

        fn sleep(&self, ms: u64) -> u64;
    }

    // context/environment api
    trait Environment {
        fn env_get(&self, key: &str);
        fn env_set(&self, key: &str, val: &str);
        fn env_mask(&self, key: &str);
        fn env_mask_all(&self);
    }

    struct Quota {
        cpu_current: u64,
        cpu_limit: u64,
        mem_current: u64,
        mem_limit: u64,
        disk_current: u64,
        disk_limit: u64,
    }

    trait Reflect {
        fn quota(&self) -> Quota;
        fn task_count(&self) -> u64;
        fn log_read(
            &self,
            level: Option<LogLevel>,
            after: Option<Time>,
            before: Option<Time>,
            limit: u64,
        ) -> (LogLevel, String, Time);
    }

    enum Request {
        Http(HttpReq),
        Fork(FnOnce(())),
        Join(u64),
    }

    enum Response {
        Http(HttpRes),
        Fork(u64),
        Join(()),
        /// does this have a strict ordering?
        Event(Event),
    }

    // copy io_uring instead of polling?
    trait Uring {
        fn ring_push(&self, req: Request) -> u64;
        fn ring_push_after(&self, req: Request, id: u64) -> u64;
        fn ring_pull(&self) -> Option<Response>;
        fn ring_cancel(&self, id: u64);
        fn ring_enter(&self);
        fn task_yield(&self); // same thing...?
    }

    trait Tasks {
        fn task_create(&self) -> u64;
        fn task_env_set(&self, id: u64, key: &str, val: &str);
        fn task_env_remove(&self, id: u64, key: &str);
        fn task_env_get(&self, id: u64, key: &str);
        fn task_env_copy(&self, id: u64, key: &str);
        fn task_env_copy_all(&self, id: u64);
        fn task_set_function(&self, id: u64, call: FnOnce());
        fn task_spawn(&self, id: u64);
        /// blocking!
        fn task_join(&self, id: u64);
        fn task_poll(&self, id: u64) -> bool;
    }

    // builtin memory thing to save some space?
    trait Memory {
        /// memory allocate
        fn malloc(&self, size: u64) -> u64;

        /// clear (zero) allocate
        fn calloc(&self, size: u64) -> u64;

        fn free(&self, ptr: u64);
    }

    trait Unwind {
        fn backtrace(&self) -> Vec<String>;
        fn unwind(&self) -> !;

        /// if (catch_start()) {
        ///     // we're unwinding!
        /// } else {
        ///     // proceed as usual
        /// }
        fn catch_start(&self) -> bool;
        fn catch_end(&self);
    }

    trait Misc {
        fn exit(&self) -> !;
    }

    trait PubSub {
        // asdf idk
        // i dont *really* want to become aws or another cloud platform
    }
}

enum Event {
    Setup,
    BeforeUpgrade,
    AfterUpgrade,
    Shutdown,
    Restart,
    Sync(MessageSync),
    Error(RedexError),
    Cron { job: String },
    Queue { id: u64, name: String },
    Uring,
}

trait Redex {
    /// returning automatically calls uring_enter?
    fn handle(&self, event: Event);
}

/// what if i made my own lang (api idk)
mod lang_api {
    trait Net {
        fn fetch(&self, method: &str, url: &str) -> RequestBuilder;
    }

    struct RequestBuilder;
    struct Request;
    struct Response;
    struct ResponseBody;

    impl RequestBuilder {
        pub fn header(self, name: &str, val: &str) -> Self;
        pub fn timeout(self, ms: u64) -> Self;
        pub fn body(self, body: Vec<u8>) -> Self;
        pub fn send(self) -> Request;
    }

    impl Request {
        pub fn poll(self) -> Result<Response, Self>;
        pub fn block(self) -> Response;
    }

    impl Response {
        pub fn status(&self) -> u16;
        pub fn headers(&self) -> HeaderMap;
        pub fn body(&self) -> Body;
    }

    impl RedexRead for ResponseBody {}

    trait RedexRead {
        fn ready(&self, min_len: u64) -> bool;
        fn read(&self, buf: &mut [u8]);
    }
}

/// what if i made my own lang
mod lang {
    // combination of haskell/lisp/rust/typescript/ocaml

    // kind of torn between wasm and building a custom language
}
