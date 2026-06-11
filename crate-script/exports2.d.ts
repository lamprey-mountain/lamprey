// FUTURE TYPES, DOESNT EXIST YET
// for use with the scripting api

import type { RunManager, RunSendable, RunMessage } from "lamprey:run";
import type { EnvManager } from "lamprey:env";
import type { NetworkManager } from "lamprey:net";
import type { StorageManager } from "lamprey:storage";
import type { ApiManager } from "lamprey:api";

declare const self: Globals;

declare global {
	export type Globals = {
		log: {
			debug(content: string, metadata: Record<string, string>): void;
			info(content: string, metadata: Record<string, string>): void;
			warn(content: string, metadata: Record<string, string>): void;
			error(content: string, metadata: Record<string, string>): void;
		}
	};

	export type Context = {
		/** manage runs
		 *
		 * can send messages to, start, stop, control, etc runs
		 */
		// TODO: split apart messaging and control. maybe have granular permissions for messaging/control?
		// TODO: should i have supervisor trees? how would it work?
		run: RunManager;

		env: EnvManager;

		net: NetworkManager;

		storage: StorageManager;

		api: ApiManager;
	};

	export type Capability =
		| { type: "env"; secrets?: string[] }
		| { type: "run" }
		// | { type: "run_supervise" } // can manage child runs
		// | { type: "run_manage" } // can manage all runs
		| { type: "net"; allow?: string[] }
		| { type: "storage" }
		| { type: "message" }
		| { type: "message_optional" }
		| { type: "api" }
		| { type: "api_event" }
		| { type: "request" };

	export type CapabilityLike = Capability;

	/** Standard TS utility to turn (A | B) into (A & B) */
	type UnionToIntersection<U> = (U extends unknown ? (x: U) => void : never) extends (
		x: infer I,
	) => void
		? I
		: never;

	/**
	 * Maps a Capability object to the slice of the Context it grants access to.
	 * e.g. { type: "fs" } -> { fs: Context['fs'] }
	 */
	type CapabilityToContext<P extends Capability> = P extends { type: infer T }
		? T extends "message" ? { message: RunMessage }
		: T extends "message_optional" ? { message?: RunMessage }
		: T extends "request" ? { request: Request }
		: T extends "api_event" ? { event: MessageSync }
		: T extends keyof Context
		? Pick<Context, T>
		: never
		: never;

	/**
	 * Takes the array T, converts each element to its context slice,
	 * then intersects them all.
	 */
	type MergedContext<T extends readonly Capability[]> = UnionToIntersection<
		CapabilityToContext<T[number]>
	>;

	export type Register = {
		/** basic input, must be manually triggered */
		onTrigger(): Input<[]>;

		/** when a message is received from another run */
		onMessage(): Input<[{ type: "message" }]>;

		/** when this process is started for the first time */
		onSpawn(): Input<[{ type: "message_optional" }]>;

		/** respond when an http request is created */
		onHttp(): Input<[{ type: "request" }], Response | Promise<Response>>;

		// TODO: maybe have onRequest, onTell here for services instead of lamprey:service module

		/** do something every once in a while */
		onCron(cron: string): Input<[]>;

		/** respond when there is an api event (MessageSync )*/
		onEvent(eventTypes?: string[]): Input<["api_event"]>;

		// TODO: maybe have onChildStopped

		// /** when this script is installed for the first time */
		// onInstall(): Input<[]>;

		// /** when this script version becomes the active version
		// *
		// * will fire again on downgrade
		// */
		// onActivate(): Input<[]>;

		// // serviceworker lifecycle events...? maybe not relevant here
		// onBlocking(): Promise<void>;
		// onShutdown(): Promise<void>;

		// /** when this script hits some quota (cpu, mem, storage) */
		// onQuota(): Input<[]>;
	};

	// TODO: rename this?
	export type Input<T extends Capability[], R = void> = {
		needsRuns(): Input<[...T, { type: "run" }]>;

		/** needs access to the network. can be restricted to certain hosts. */
		needsHost<H extends string[]>(
			allow?: H,
		): Input<[...T, { type: "net"; allow: H }]>;

		/** needs access to a protected configuration secret */
		needsSecret<N extends string[]>(
			names?: N,
		): Input<[...T, { type: "env"; secrets: N }]>;

		needsStorage(): Input<[...T, { type: "storage" }]>;

		needsSharedStorage(): Input<[...T, { type: "storage" }]>;

		id(id: string): Input<T>;
		label(id: string): Input<T>;

		run(call: (ctx: MergedContext<T>) => R): void;
	};

	// export type ApiInput<T extends Capability[], R = void> = Input<T, R> & {
	// 	/** authenticate as another application using ia token from env var */
	// 	token(env: string): Input<[...T, { type: "api" }]>;

	// 	/** login via application id. script owner  */
	// 	applicationId(appId: string): Input<[...T, { type: "api" }]>;

	// 	/** same as applicationId but with env */
	// 	applicationIdFromEnv(env: string): Input<[...T, { type: "api" }]>;
	// };

	export interface Script {
		// deprecate `name: string`, require nesting `metadata`
		metadata: ScriptMetadata,

		/** register inputs. if there are none, this is a library. */
		register?: (ctl: Register) => void;
	}

	export type ScriptMetadata = {
		name: string;
		description?: string;
		homepage_url?: string; // enforce url?
		authors?: ScriptAuthor[];
		version?: string; // enforce semver?
		license?: string; // enforce spdx?

		// extra fields?
		id?: string; // if this exists when script is created, replace/update existing script with this id
		scriptName?: string; // name is human readable, this is for identifying scripts (needs better name)
	}

	export type ScriptAuthor = {
		name: string;
		user?: ScriptAuthorOrigin;
		url?: string;
	};

	export type ScriptAuthorOrigin = {
		origin_id: string;
		hostname: string;
	};

	/** helper to define type safe scripts */
	export const defineScript: (script: Script) => Script;

	// /** helper to define type safe services */
	// export const defineService: (service: Service) => Service;

	// /** helper to define type safe libraries */
	// export const defineLibrary: (script: Omit<Script, "register">) => Script;
}

declare module "lamprey:env" {
	/** access config */
	export class EnvManager<AllowedKeys extends string = string, AllowedSecrets extends string = string> {
		/** lookup a public env value or non opaque secret */
		get(name: AllowedKeys): string | null;

		/** lookup an opaque env secret */
		getSecret(name: AllowedSecrets): EnvSecret | null;
	}

	/** can be used in the api */
	export class EnvSecret {
		/** extract data if readable */
		read(): string;
	}

	export type EnvDisposition =
		| "template" // public + cloning the script also copies over this value
		| "public" // all runs can read this
		| "secret" // access must be requested
		| "opaque"; // access must be requested, code cannot read data
}

export type ScriptId = string & { readonly __brand: "ScriptId" };
export type RunId = string & { readonly __brand: "RunId" };

/** tools for managing other runs */
// NOTE: maybe rename to "process" for more standard terminology
declare module "lamprey:run" {
	// TODO: i use rquickjs (rust quickjs bindings), maybe look at what can be sent
	export type RunSendable =
		| string | number | boolean | null | undefined
		| Uint8Array | Date | RegExp
		| RunSendable[]
		| { [key: string]: RunSendable }
		| Map<RunSendable, RunSendable>
		| Set<RunSendable>;

	export type RunMessage = {
		data: RunSendable;
		source: RunId;
		timestamp: Date;
	};

	// maybe return this in ctx.runs instead of making people do new RunContext
	export class RunManager {
		/** lookup your own run */
		lookupSelf(): SelfProcess;

		/** lookup any run spawned from this script */
		lookupScript(scriptId: string): Run | null;
		// lookupScript(scriptId: string): RunSet | null;
		// lookupScriptByName(scriptName: string): RunSet | null;

		/** lookup a specific run */
		lookupRun(runId: string): Run | null;

		// maybe make async
		spawn(scriptId: string, data?: RunSendable): Run;

		// // if child dies, kill parent
		// spawnLink(scriptId: string, data?: RunSendable): Run;

		// // get signal when child dies
		// spawnMonitor(scriptId: string, data?: RunSendable): Run;
	}

	// NOTE: unsure how exactly this would work?
	// NOTE: can this be empty?
	/** a set of multiple runs */
	export class RunSet {
		/** send a message to all runs in this set */
		broadcast(msg: RunSendable): void;

		/** pick an arbitrary run */
		arbitrary(): Run;

		readonly scriptId: string;
	}

	// TODO: i could hide send/stop depending on context perms but typing that sounds painful
	export class Run {
		send(msg: RunSendable): void; // may fail (erlang semantics)
		stop(): void;
		readonly scriptId: ScriptId;
		readonly id: RunId;
	}

	export class SelfProcess extends Run {
		receive(timeout?: number): Promise<RunMessage>;
	}
}

/** basically gen_server */
declare module "lamprey:service" {
	// NOTE: i could make one script/run = one service and have onRequest/onTell instead
	// FIXME: no explicit any
	type ServiceReq<S> = S extends Service<any, unknown, infer Req, unknown, unknown> ? Req : never;
	type ServiceTell<S> = S extends Service<any, unknown, unknown, infer Tell, unknown> ? Tell : never;
	type ServiceRes<S> = S extends Service<any, unknown, unknown, unknown, infer Res> ? Res : never;

	export abstract class Service<
		Context extends MergedContext<[{ type: "run" }]>,
		State,
		Tell extends RunSendable,
		Req extends RunSendable,
		Res extends RunSendable,
	> {
		abstract init(ctx: Context): State; // TODO: make context work
		abstract handleRequest(msg: Req, state: State): [Res, state: State];
		abstract handleTell(msg: Tell, state: State): State;
	}

	export class ServiceClient<S extends Service<any, unknown, unknown, unknown, unknown>> {
		request(msg: ServiceReq<S>, timeout?: number): Promise<ServiceRes<S>>;
		tell(msg: ServiceTell<S>): void;
	}

	// maybe some more erlang stuff
	// export type SupervisorStrategy =
	// 	| { type: "one_for_one" }  // restart only the crashed child
	// 	| { type: "one_for_all" }  // restart all children if one crashes
	// 	| { type: "rest_for_one" } // restart crashed child + all started after it

	// export type SupervisorConfig = {
	// 	strategy: SupervisorStrategy;
	// 	maxRestarts: number;   // max N restarts...
	// 	maxWindow: number;     // ...within this many ms (then supervisor itself exits)
	// };

	// what else needs to be here?
	// one issue with an elixir-style api is that scripts/runs aren't really long lived processes. they're triggered (receive input), do some processing, then exit.
}

/** network types */
declare module "lamprey:net" {
	export class NetworkManager {
		fetch(req: Request): Promise<Response>;

		// TODO: in the future
		// connect(): Promise<...>;
		// connectTls(): Promise<...>;
		// somethingUdp(): Promise<...>;
	}

	/** opaque container representing an ip address. allows using ip addrs for moderation without leaking them */
	export class IpAddress {
		equals(other: IpAddress): boolean;
	}
}

/** http types */
declare module "lamprey:http" {
	// NOTE: i should probably make these use actual http methods
	export class Request {
		readonly url: string;
		readonly method: "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | string;
		readonly redirect: string;
		readonly headers: Headers;

		// compat?
		readonly statusText: string;

		// extra?
		readonly path: string;

		// readonly body: ReadableStream | null;
		// json(): Promise<any>;
		// text(): Promise<string>;
		// blob, arraybuffer
	}

	export class FetchInit extends Request {
		// cache?: no-store, no-cache
	}

	export class Response {
		readonly body: unknown;
		readonly headers: unknown;
		readonly status: number;
		readonly url: string;

		// compat?
		readonly statusText: string;
		readonly bodyUsed: boolean;
		readonly ok: boolean;
		readonly redirected: boolean;

		// extra?
		// network info (ip addr)

		// constructor(body?: BodyInit, init?: ResponseInit);
		// static json(data: any, init?: ResponseInit): Response;

		// readonly body: ReadableStream | null;
		// json(): Promise<any>;
		// text(): Promise<string>;
		// blob, arraybuffer
	}

	export class Headers {
		// copy web standards
		// append, delete, get, has, set, entries, keys, values, forEach
		// also getSetCookie?
	}

	export class Router {
		// write server router in rust for performance
	}

	// probably copy https://developers.cloudflare.com/workers/runtime-apis/fetch/
}

/** async utilities */
declare module "lamprey:async" {
	/** attempt to cancel a promise after a timeout */
	// this is non standard compared to abortcontroller though
	export const timeout: <T>(p: Promise<T>, timeout: number) => Promise<T>;
}

/** storage utilities */
declare module "lamprey:storage" {
	// copy https://docs.deno.com/api/deno/~/Deno.KvKeyPart
	export type KvKeyPart = string | Uint8Array | number | bigint | boolean;
	export type KvKey = KvKeyPart[];
	export type KvValue = any;
	export type KvNumber = number | bigint;
	export type KvVersion = bigint;

	export class StorageManager {
		open(name: string): Promise<Store>;
		list(): Promise<Store[]>;
	}

	export class Store {
		// managing store
		configure(config: StoreConfig): Promise<void>;
		delete(): Promise<void>;
		count(): Promise<number>; // number of entries

		// managing indexes
		createIndex(create: CreateIndex): Promise<Index>;
		index(name: string): Promise<Index>;
		indexes(): Promise<Index[]>;

		// managing data
		read(): ReadTransaction;
		write(): WriteTransaction;
		createSnapshot(create: CreateSnapshot): Promise<Snapshot>;
		watch(prefix: KvKey, callback: (entry: Entry) => void): Watcher;

		// shortcuts
		insert(key: KvKey, value: KvValue): this;
		delete(key: KvKey): this;
		get(key: KvKey): Promise<KvValue>;
		lookup(index: string, data: KvValue): Promise<KvKey>;
		entry(key: KvKey): Promise<Entry>;
		scan(): Scanner<AsyncIterator<Entry>>;
		scanSync(): Scanner<Iterator<Entry>>;

		readonly name: string;
	}

	export type StoreConfig = {
		consistency: "strict" | "eventual";
	}

	// generic over asyncness?
	// export class ReadTransaction2<S> {
	// 	get(key: KvKey): S<KvValue>;
	// }
	// Readtransaction2<T => T>
	// Readtransaction2<T => Promise<T>>

	export class ReadTransaction {
		get(key: KvKey): Promise<KvValue>;
		entry(key: KvKey): Promise<Entry>;
		scan(): Scanner<AsyncIterator<Entry>>;
		index(name: string): Promise<Index>;
	}

	export class ReadTransactionSync {
		get(key: KvKey): KvValue;
		entry(key: KvKey): Entry;
		scan(): Scanner<Iterator<Entry>>;
		index(name: string): Index;
	}

	export class Scanner<I> {
		prefix(key: KvKey): this;
		start(key: KvKey): this;
		end(key: KvKey): this;
		reverse(reversed?: boolean): this;

		// TODO: design types to somehow prevent calling above methods once this is called
		// TODO: maybe allow iterating directly instead of requiring users to call .iter()
		iter(): I;
	}

	export class Entry {
		readonly data: KvValue;
		readonly version: KvVersion;
		readonly timestamp: Date;
	}

	export class WriteTransaction {
		// maybe there's some clean way to merge these?
		read(): ReadTransaction;
		readForUpdate(): ReadTransaction;
		readSync(): ReadTransactionSync;
		readSyncForUpdate(): ReadTransactionSync;

		check(key: KvKey, matches: KvValue): this; // fail transaction if doesnt match
		checkVersion(key: KvKey, matches: KvVersion): this; // fail transaction if doesnt match
		swap(key: KvKey, matches: KvValue, value: KvValue): this;
		swapVersion(key: KvKey, matches: KvVersion, value: KvValue): this;
		insert(key: KvKey, value: KvValue): this;
		delete(key: KvKey): this;
		sum(key: KvKey, n: KvNumber): this; // set the value at k to existing + n
		max(key: KvKey, n: KvNumber): this; // set the value at k to max(existing, n)
		min(key: KvKey, n: KvNumber): this; // set the value at k to min(existing, n)

		commit(): Promise<CommitResult>; // maybe throw on failure?
		rollback(): Promise<void>;
	}

	export type CommitResult = {
		ok: true;
		version: KvVersion
	} | {
		ok: false;
	};

	export type CreateSnapshot = {
		label: string;
	}

	// can be imported/exported from ui
	export class Snapshot {
		delete(): Promise<void>;
		read(): ReadTransaction;
		readonly timestamp: Date;
	}

	export class Watcher {
		disconnect(): void;
	}

	export type CreateIndex = {
		name: string,
		prefix?: KvKey,
		extract: (val: KvValue) => KvValue, // extract the value to index on
		constrain?: (val: KvValue) => boolean, // apply additional validation
		filter?: (val: KvValue) => boolean, // only index these values
		unique?: boolean,
	}

	// TODO: also offer a sync version?
	export class Index {
		count(): Promise<number>;
		delete(): Promise<void>;
		readonly label?: string;

		lookup(data: KvValue): Promise<KvKey>;
		get(data: KvValue): Promise<KvValue>;
		entry(data: KvValue): Promise<Entry>;
		scan(): IndexScanner<Iterator<IndexEntry>>;

		// for non-unique indexes
		// (maybe make this an async iterator?)
		lookupAll(data: KvValue): Promise<KvKey[]>;
		getAll(data: KvValue): Promise<KvValue[]>;
		entryAll(data: KvValue): Promise<Entry[]>;
	}

	export class IndexEntry extends Entry {
		readonly indexData: KvValue;
	}

	export class IndexScanner<I> {
		start(key: KvValue): this;
		end(key: KvValue): this;
		reverse(reversed?: boolean): this;
		iter(): I;
	}

	// also maybe have some way to use this as a queue
	// also maybe have some way to set a ttl
}

declare module "lamprey:api" {
	// TODO: get these from ts-sdk
	type MessageSync = any;

	export class ApiManager {
		rooms: ApiRoom;
		channels: ApiChannels;
		users: ApiUsers;
		// etc...
	}

	export class ApiRoom {
		roles: Map<string, ApiRole>;
		channels: ApiChannels;
	}

	export class ApiChannels extends Map<string, ApiChannel> {
		create(): Promise<void>;
	}

	export class ApiRooms extends Map<string, ApiRoom> { }
	export class ApiUsers extends Map<string, ApiUser> { }

	export class ApiChannel {
		messageList(): Promise<void>;
		messageGet(): Promise<void>;
		send(): Promise<void>;
		edit(): Promise<void>;
		delete(): Promise<void>;
	}

	export class ApiMessage {
		edit(): Promise<void>;
		delete(): Promise<void>;

		pin(): Promise<void>;
		unpin(): Promise<void>;

		reactionAdd(): Promise<void>;
		reactionList(): Promise<void>;
		reactionRemove(): Promise<void>;
		reactionRemoveEmoji(): Promise<void>;
		reactionRemoveAll(): Promise<void>;
	}

	export class ApiRole {
		// etc...
	}

	export class ApiUser {
		// etc...
	}

	// copy most of the sdk here i guess
	// maybe i can reuse code between external and internal code (outside scripts and inside the script runtime)

	// should probably have permission resolution logic utils
	export type Permission = "MessageCreate"; // etc...
	export class Permissions {
		has(perm: Permission): boolean;
		rank(): number;
		// what else...?
	}

	// maybe copy some stuff from discordjs
}

/** streaming html parser */
declare module "lamprey:html" {
	// for url unfurler
	// copy stuff from html5ever and that old unfurl test thing i had
}

declare module "script:uuid-here" {
	// import other scripts as libraries
}

declare module "lamprey:other-random-stuff" {
	// everything should be replayable
	// - Math.random() should be seeded
	// - Date.now() should be locked to the date when it was evaluated
	// - ...what else?

	// maybe allow using oauth to authenticate users

	// dynamic permissions (how do they work? who gets prompted?)
}

