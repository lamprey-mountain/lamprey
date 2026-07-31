// manager<type, key (id of type), resolvable (something that can be resolved to a key/type)>
export abstract class Manager<T, K, R = never> {
	abstract resolve(it: T): T;
	abstract resolve(it: K | R): T | undefined;
	abstract resolveId(it: T | K): K;
	abstract resolveId(it: R): K | undefined;
}
