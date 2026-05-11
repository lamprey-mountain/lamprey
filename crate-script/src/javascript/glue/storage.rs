use rquickjs::{
    class::{Trace, Tracer},
    Ctx, Function, JsLifetime, Persistent,
};

/// manages key value stores
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct StorageManager {
    // TODO
}

/// a single key value store
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct Store {
    // TODO: store reference, name
}

/// configuration for a store
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct StoreConfig {
    /// consistency mode
    pub consistency: String, // TODO: make enum
}

/// a read-only transaction
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct ReadTransaction {
    // TODO: store reference
}

/// a transactional write operations builder
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct WriteTransaction {
    // TODO: store reference, pending operations
}

/// result of a commit operation
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct CommitResult {
    /// whether the commit succeeded
    pub ok: bool,

    /// version number if successful
    pub version: Option<String>,
}

/// watches for changes on a key prefix
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct Watcher {
    // TODO: watch handle
}

/// an entry in storage (key-value pair with metadata)
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct Entry {
    // TODO: data, version, timestamp
}

/// a storage index
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct Index {
    // TODO: store reference, index name
}

/// an entry within an index scan
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct IndexEntry {
    // TODO: extends Entry with indexData
}

/// scanner for index scans
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct IndexScanner {
    // TODO: reference to scan iterator
}

/// scanner for store scans
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct Scanner {
    // TODO: reference to scan iterator
}

/// a point-in-time read snapshot
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct Snapshot {
    // TODO: snapshot reference, timestamp
}

/// configuration for creating an index
#[rquickjs::class]
#[derive(Clone, JsLifetime)]
pub struct CreateIndex {
    /// index name
    pub name: String,

    /// optional key prefix
    pub prefix: String, // maybe should be Option and not String

    /// function to extract index value
    pub extract: Option<Persistent<Function<'static>>>,

    /// optional constraint function
    pub constrain: Option<Persistent<Function<'static>>>,

    /// optional filter function
    pub filter: Option<Persistent<Function<'static>>>,

    /// whether the index is unique
    pub unique: bool,
}

/// configuration for creating a snapshot
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct CreateSnapshot {
    /// snapshot label
    pub label: String,
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl StorageManager {
    /// open a named store
    fn open<'js>(&self, _name: String, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// list all available stores
    fn list<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl Store {
    /// configure store settings
    fn configure<'js>(
        &self,
        _config: rquickjs::Object<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// delete this store
    fn delete<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// count number of entries
    fn count<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// create an index on this store
    fn create_index<'js>(
        &self,
        _create: rquickjs::Object<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// get an existing index
    fn index<'js>(&self, _name: String, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// list all indexes
    fn indexes<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// start a read transaction
    fn read<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// start a write transaction
    fn write<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// begin a snapshot builder
    fn create_snapshot<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// watch for changes on a key prefix
    fn watch<'js>(
        &self,
        _prefix: rquickjs::Value<'js>,
        _callback: rquickjs::Function<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// insert a key-value pair (fluent)
    fn insert<'js>(
        &mut self,
        _key: rquickjs::Value<'js>,
        _value: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// delete a key (fluent)
    fn delete_key<'js>(
        &mut self,
        _key: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// get value at key
    fn get<'js>(
        &self,
        _key: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// lookup a key by index
    fn lookup<'js>(
        &self,
        _index: String,
        _data: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// get entry at key
    fn entry<'js>(
        &self,
        _key: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// scan entries (async iterator)
    fn scan<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// scan entries (sync iterator)
    fn scan_sync<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// the store name
    fn name<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<String> {
        todo!()
    }
}

#[rquickjs::methods]
impl StoreConfig {}

#[rquickjs::methods]
impl ReadTransaction {
    /// get value at key
    fn get<'js>(
        &self,
        _key: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// get entry at key
    fn entry<'js>(
        &self,
        _key: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// scan entries
    fn scan<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// get an index
    fn index<'js>(&self, _name: String, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }
}

#[rquickjs::methods]
impl WriteTransaction {
    /// start a read within this transaction
    fn read<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// start a read-within-write transaction
    fn read_for_update<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// check that a key matches expected value
    fn check<'js>(
        &self,
        _key: rquickjs::Value<'js>,
        _matches: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// check that a key matches expected version
    fn check_version<'js>(
        &self,
        _key: rquickjs::Value<'js>,
        _matches: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// swap key: only if matches, set to value
    fn swap<'js>(
        &self,
        _key: rquickjs::Value<'js>,
        _matches: rquickjs::Value<'js>,
        _value: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// swap with version check
    fn swap_version<'js>(
        &self,
        _key: rquickjs::Value<'js>,
        _matches: rquickjs::Value<'js>,
        _value: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// insert a key-value pair
    fn insert<'js>(
        &self,
        _key: rquickjs::Value<'js>,
        _value: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// delete a key
    fn delete_key<'js>(
        &self,
        _key: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// atomic sum: value = existing + n
    fn sum<'js>(
        &self,
        _key: rquickjs::Value<'js>,
        _n: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// atomic max: value = max(existing, n)
    fn max<'js>(
        &self,
        _key: rquickjs::Value<'js>,
        _n: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// atomic min: value = min(existing, n)
    fn min<'js>(
        &self,
        _key: rquickjs::Value<'js>,
        _n: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// commit the transaction
    fn commit<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// rollback the transaction
    fn rollback<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }
}

#[rquickjs::methods]
impl CommitResult {
    /// check if commit was successful
    fn ok(&self) -> bool {
        self.ok
    }

    /// version if successful
    fn version(&self) -> Option<String> {
        self.version.clone()
    }
}

#[rquickjs::methods]
impl Snapshot {
    /// delete this snapshot
    fn delete<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// start a read transaction on this snapshot
    fn read<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// when this snapshot was created
    fn timestamp<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }
}

#[rquickjs::methods]
impl Watcher {
    /// stop watching
    fn disconnect(&self) {
        todo!()
    }
}

#[rquickjs::methods]
impl Entry {
    /// the stored value
    fn data<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// version of this entry
    fn version<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// timestamp of this entry
    fn timestamp<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }
}

#[rquickjs::methods]
impl CreateIndex {}

#[rquickjs::methods]
impl Index {
    /// count indexed entries
    fn count<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// delete this index
    fn delete<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// the index label
    fn label<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<Option<String>> {
        todo!()
    }

    /// lookup a key by indexed value
    fn lookup<'js>(
        &self,
        _data: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// get value by indexed value
    fn get<'js>(
        &self,
        _data: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// get entry by indexed value
    fn entry<'js>(
        &self,
        _data: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// scan indexed entries
    fn scan<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// lookup all keys by indexed value (non-unique)
    fn lookup_all<'js>(
        &self,
        _data: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// get all values by indexed value (non-unique)
    fn get_all<'js>(
        &self,
        _data: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// get all entries by indexed value (non-unique)
    fn entry_all<'js>(
        &self,
        _data: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }
}

#[rquickjs::methods]
impl IndexEntry {
    /// the data that matched the index
    fn index_data<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl IndexScanner {
    /// filter by start key
    fn start<'js>(
        &self,
        _key: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// filter by end key
    fn end<'js>(
        &self,
        _key: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// reverse the scan order
    fn reverse<'js>(
        &self,
        _reversed: bool,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// return the iterator
    fn iter<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl Scanner {
    /// filter by prefix
    fn prefix<'js>(
        &self,
        _key: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// filter by start key
    fn start<'js>(
        &self,
        _key: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// filter by end key
    fn end<'js>(
        &self,
        _key: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// reverse the scan order
    fn reverse<'js>(
        &self,
        _reversed: bool,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// return the iterator
    fn iter<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }
}

// manually implement Trace because Persistent doesn't implement it
// since Persistent is a root, we don't need to visit it during tracing
impl<'js> Trace<'js> for CreateIndex {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
}

#[rquickjs::module(rename = "lamprey:storage")]
pub mod inner {
    pub use super::{
        CommitResult, CreateIndex, CreateSnapshot, Entry, Index, IndexEntry, IndexScanner,
        ReadTransaction, Scanner, Snapshot, StorageManager, Store, StoreConfig, Watcher,
        WriteTransaction,
    };
}
