import createFetch, { FetchResponse, MaybeOptionalInit } from "openapi-fetch";
import {
	Emitter,
	type Http,
	type MessageClient,
	type MessageEnvelope,
	type paths,
} from "ts-sdk";
import type { ClientOptions } from "./client";
import { Syncer } from "./syncer";

export type BackendEvents = {
	sync: MessageEnvelope;

	// error: Error;
	// ready: MessageReady;
	// state: SyncerState;
};

// TODO: fix type gymnastics
export type HttpMethod =
	| "get"
	| "put"
	| "post"
	| "delete"
	| "options"
	| "head"
	| "patch"
	| "trace";

type PathParams<
	P extends keyof paths,
	M extends keyof paths[P],
> = paths[P][M] extends { parameters: { path: infer Params } } ? Params : never;

type QueryParams<
	P extends keyof paths,
	M extends keyof paths[P],
> = paths[P][M] extends { parameters: { query: infer Query } } ? Query : never;

type HeaderParams<
	P extends keyof paths,
	M extends keyof paths[P],
> = paths[P][M] extends { parameters: { header: infer Headers } }
	? Headers
	: never;

type RequestBody<
	P extends keyof paths,
	M extends keyof paths[P],
> = paths[P][M] extends {
	requestBody: { content: { "application/json": infer Body } };
}
	? Body
	: never;

type ResponseBody<
	P extends keyof paths,
	M extends keyof paths[P],
> = paths[P][M] extends {
	responses: { 200: { content: { "application/json": infer Res } } };
}
	? Res
	: paths[P][M] extends {
				responses: { 201: { content: { "application/json": infer Res } } };
			}
		? Res
		: unknown;

export type Request<
	Path extends keyof paths = keyof paths,
	Method extends keyof paths[Path] = keyof paths[Path],
> = {
	path: Path;
	method: Method;
} & (PathParams<Path, Method> extends never
	? {}
	: { params: PathParams<Path, Method> }) &
	(QueryParams<Path, Method> extends never
		? {}
		: { query: QueryParams<Path, Method> }) &
	(HeaderParams<Path, Method> extends never
		? {}
		: { headers: HeaderParams<Path, Method> }) &
	(RequestBody<Path, Method> extends never
		? {}
		: { body: RequestBody<Path, Method> });

export type UntypedRequest = {
	path: string;
	method: HttpMethod;
	params?: Record<string, string>;
	query?: Record<string, string>;
	headers?: Record<string, string>;
	body?: unknown;
};

// export type Response<
// 	Path extends keyof paths,
// 	Method extends keyof paths[Path],
// > = paths[Path][Method] extends { responses: infer R }
// 	? R extends { 200: { content: { "application/json": infer D } } }
// 		? D
// 		: R extends { 201: { content: { "application/json": infer D } } }
// 			? D
// 			: R extends { 202: { content: { "application/json": infer D } } }
// 				? D
// 				: R extends { 204: any }
// 					? void
// 					: unknown
// 	: unknown;

// const b = null as unknown as Backend;
// b.fetch({ method: "post", path: "/api/v1/.well-known/oauth-authorization-server" });

export abstract class Backend extends Emitter<BackendEvents> {
	// abstract fetch<
	// 	Path extends keyof paths,
	// 	Method extends keyof paths[Path]
	// >(req: Request<Path, Method>): Response<Path, Method>;
	abstract fetch(req: UntypedRequest): Promise<Response>;

	abstract send(msg: MessageClient): void;
}

export { SharedBackend } from "./shared-worker/client.ts";

// TODO: move to another file?
export class DirectBackend extends Backend {
	private syncer: Syncer;
	private http: Http;

	constructor(o: ClientOptions & { token: string }) {
		super();

		const syncer = new Syncer({
			apiUrl: o.apiUrl,
			token: o.token,

			compress: "deflate",
			format: "msgpack",
		});
		syncer.on("sync", (m) => this.emit("sync", m));
		syncer.on("error", () => {
			/* TODO */
		});
		syncer.on("ready", () => {
			/* TODO */
		});
		this.syncer = syncer;

		const http = createFetch<paths>({
			baseUrl: o.apiUrl,
		});

		http.use({
			onRequest(r) {
				if (o.token) {
					r.request.headers.set("authorization", `Bearer ${o.token}`);
				}
				return r.request;
			},
		});

		this.http = http;
	}

	async fetch(req: UntypedRequest): Promise<Response> {
		// TODO: fix types
		// TODO: deduplicate requests
		const { response, error } = await this.http.request(
			req.method as any,
			req.path as any,
			{
				params: {
					path: req.params,
					query: req.query,
					headers: req.headers,
				},
				body: req.body,
			},
		);

		if (response.ok) {
			return response;
		} else {
			throw error;
		}
	}

	send(msg: MessageClient): void {
		this.syncer.send(msg);
	}
}
