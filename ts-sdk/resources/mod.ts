// manager<type, key (id of type), resolvable (something that can be resolved to a key/type)>
export abstract class Manager<T, K, R = never> {
	abstract resolve(it: T): T;
	abstract resolve(it: K | R): T | undefined;
	abstract resolveId(it: T | K): K;
	abstract resolveId(it: R): K | undefined;
}

// // how to fetch a resource
// abstract class Layer {
// 	// fetch()
// 	// put() // handle resource update (eg. update cache)
// 	// maybe don't include put? data only flows through fetch() one way
// }

// class LayerIdbCache extends Layer {} // try to load from idb
// class LayerSharedWorker extends Layer {} // try to load from shared worker
// class LayerFetch extends Layer {} // fetch from network

// class Loader<T> {}

// cascading layers

// in SharedWorker
// - indexeddb
// - fetch

// in Window
// - local cache
// - sharedworker
