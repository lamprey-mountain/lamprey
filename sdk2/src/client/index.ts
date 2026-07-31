import { Emitter } from "../core/events";
import { RoomManager } from "../resources/room";
import type { schemas } from "../schema";

export type LampreyOptionsResolved = {
	apiUrl: string;
	cdnUrl?: string;

	backend: "direct" | "sharedWorker";
	storage?: "sqlite" | "idb";
	cache: "enable" | "disable"; // TODO: actual cache options
	// token?: string;
	// TODO: allow passing custom fetch impl

	// format: "json" | "msgpack";
	// compress?: "deflate";
};

// if cache is "disable", don't save anything in `FooManager`s

export type LampreyOptions = Partial<LampreyOptionsResolved>;

function resolveOptions(opts: LampreyOptions): LampreyOptionsResolved {
	return {
		apiUrl: opts.apiUrl ?? "https://chat.celery.eu.org",
		cdnUrl: opts.cdnUrl,
		backend: opts.backend ?? "direct",
		cache: opts.cache ?? "enable",
		storage: opts.storage,
	};
}

export type LampreyEvents = {
	sync(sync: schemas["MessageSync"]): void;
	// ready(ready: schemas[""]): void;

	// onReady: (event: MessageReady) => void;
	// onSync: (event: MessageSync, raw: MessageEnvelope) => void;
	// onError?: (error: Error) => void;
	// onSend?: (data: unknown) => void;
	// onMessage?: (raw: MessageEnvelope) => void;

	roomCreate(message: schemas["Room"]): void;

	messageCreate(message: schemas["Message"]): void;
	messageUpdate(message: schemas["Message"]): void;
	messageDelete(channelId: string, messageId: string): void;
};

export class Lamprey extends Emitter<LampreyEvents> {
	public options: LampreyOptionsResolved;
	// public http: Http;
	// private backend: Backend;
	// [ClientBackend]: Backend;

	public readonly rooms: RoomManager;
	// channels
	// users
	// media
	// voice (maybe?)
	// auth (for logging in)

	constructor(options: LampreyOptions) {
		const o = resolveOptions(options);

		super();

		this.options = o;
		this.rooms = new RoomManager(this);
		// TODO: more managers...
	}

	/** start the client. takes an existing auth token, otherwise creates a new session */
	public async start(token?: string) {
		new Syncer();
		// TODO
	}

	/** stop the client */
	public stop() {
		// TODO
	}

	// // TODO: move to auth service/manager?
	// /** logout and invalidate auth token */
	// public async logout() {
	//   // TODO
	// }

	// /** Send a message to the sync server, queueing if not connected */
	// send: (data: MessageClient) => void;
}

// TODO: impl backends?
export type BackendEvents = {
	// sync: MessageEnvelope;
	// error: Error;
	// ready: MessageReady;
	// state: SyncerState;
	// disconnect/reconnect
};

export abstract class Backend extends Emitter<BackendEvents> {
	// abstract fetch<
	// 	Path extends keyof paths,
	// 	Method extends keyof paths[Path]
	// >(req: Request<Path, Method>): Response<Path, Method>;
	// abstract fetch(req: UntypedRequest): Promise<Response>;
	// abstract send(msg: MessageClient): void;
}

// export class DirectBackend extends Backend { }
// export class SharedWorkerBackend extends Backend { }

// TODO: copy ts-sdk/client/syncer.ts here
