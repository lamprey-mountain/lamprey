import type { Middleware } from "openapi-fetch";

export type RatelimitScope = "global" | "shared" | "user" | "session";

export type RatelimitInfo = {
	/** seconds to wait before retrying */
	retry_after: number;
	/** opaque id of the bucket that was hit */
	bucket?: string;
	/** maximum number of requests allowed in the bucket */
	limit?: number;
	/** requests remaining in the bucket */
	remaining?: number;
	/** what the bucket is scoped to */
	scope?: RatelimitScope;
};

export type RetryOptions = {
	/** total number of attempts, including the first one */
	retries?: number;
	/** fallback delay for 5xx/network errors */
	baseDelay?: number;
	/** upper bound for any single wait */
	maxDelay?: number;
	/** called whenever a 429 is encountered */
	onRatelimit?: (info: RatelimitInfo) => void;
};

export type HttpResult<T> = {
	data?: T;
	error?: unknown;
	response: Response;
};

const DEFAULT_RETRIES = 3;
const DEFAULT_BASE_DELAY = 1000;
const DEFAULT_MAX_DELAY = 30_000;

function isObject(value: unknown): value is Record<string, unknown> {
	return typeof value === "object" && value !== null;
}

function asNumber(value: unknown): number | undefined {
	if (typeof value === "number") {
		return Number.isFinite(value) ? value : undefined;
	}
	if (typeof value === "string" && value.trim() !== "") {
		const parsed = Number(value);
		return Number.isFinite(parsed) ? parsed : undefined;
	}
	return undefined;
}

function asScope(value: unknown): RatelimitScope | undefined {
	switch (value) {
		case "global":
		case "shared":
		case "user":
		case "session":
			return value;
		default:
			return undefined;
	}
}

function parseRetryAfterHeader(value: string | null): number | undefined {
	if (!value) return undefined;

	const seconds = asNumber(value);
	if (seconds !== undefined) {
		return Math.max(0, seconds * 1000);
	}

	const date = Date.parse(value);
	if (!Number.isNaN(date)) {
		return Math.max(0, date - Date.now());
	}

	return undefined;
}

function ratelimitBody(value: unknown): Partial<RatelimitInfo> {
	if (!isObject(value)) return {};

	const source = isObject(value.ratelimit) ? value.ratelimit : value;
	if (!isObject(source)) return {};

	return {
		retry_after: asNumber(source.retry_after),
		bucket: typeof source.bucket === "string" ? source.bucket : undefined,
		limit: asNumber(source.limit),
		remaining: asNumber(source.remaining),
		scope: asScope(source.scope),
	};
}

function ratelimitHeaders(response: Response): Partial<RatelimitInfo> {
	const headers = response.headers;
	return {
		bucket: headers.get("x-ratelimit-bucket") ?? undefined,
		limit: asNumber(headers.get("x-ratelimit-limit")),
		remaining: asNumber(headers.get("x-ratelimit-remaining")),
		scope: asScope(headers.get("x-ratelimit-scope")),
	};
}

export function parseRetryAfter(value: string | null): number | undefined {
	return parseRetryAfterHeader(value);
}

export function ratelimitFromError(error: unknown): RatelimitInfo | undefined {
	const body = ratelimitBody(error);
	if (body.retry_after === undefined) return undefined;
	return { ...body, retry_after: body.retry_after };
}

export function isRatelimitError(error: unknown): boolean {
	if (!isObject(error)) return false;
	if (error.code === "Ratelimit") return true;
	return isObject(error.ratelimit);
}

export function ratelimitFromResponse(
	response: Response,
	body?: unknown,
): RatelimitInfo {
	const fromBody = ratelimitBody(body);
	const fromHeaders = ratelimitHeaders(response);

	const retryAfter =
		fromBody.retry_after ??
		(parseRetryAfterHeader(response.headers.get("retry-after")) ?? 0) / 1000;

	return {
		retry_after: Math.max(0, retryAfter),
		bucket: fromBody.bucket ?? fromHeaders.bucket,
		limit: fromBody.limit ?? fromHeaders.limit,
		remaining: fromBody.remaining ?? fromHeaders.remaining,
		scope: fromBody.scope ?? fromHeaders.scope,
	};
}

export function retryAfterMs(
	response: Response,
	body?: unknown,
): number | undefined {
	const fromBody = ratelimitBody(body).retry_after;
	if (fromBody !== undefined) {
		return Math.max(0, fromBody * 1000);
	}
	return parseRetryAfterHeader(response.headers.get("retry-after"));
}

function errorResponse(error: unknown): Response | undefined {
	if (isObject(error) && error.response instanceof Response) {
		return error.response;
	}
	return undefined;
}

function shouldRetry(response: Response | undefined, error: unknown): boolean {
	if (response) {
		return response.status === 429 || response.status >= 500;
	}
	// no response means the request never made it (or was aborted); retry like
	// the old hand-rolled implementation did
	return !isRatelimitError(error);
}

function fallbackDelay(
	attempt: number,
	baseDelay: number,
	maxDelay: number,
): number {
	const delay = baseDelay * 2 ** attempt + Math.random() * 100;
	return Math.min(delay, maxDelay);
}

function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

export function ratelimitMiddleware(options: RetryOptions = {}): Middleware {
	const retries = Math.max(0, options.retries ?? DEFAULT_RETRIES);
	const baseDelay = options.baseDelay ?? DEFAULT_BASE_DELAY;
	const maxDelay = options.maxDelay ?? DEFAULT_MAX_DELAY;
	const clones = new Map<string, Request>();

	return {
		onRequest({ id, request }) {
			// stash a clone before fetch consumes the body; the same request is
			// reused for every retry
			clones.set(id, request.clone());
		},
		onError({ id }) {
			// fetch rejected before onResponse could clean up
			clones.delete(id);
		},
		async onResponse({ id, response, options: clientOptions }) {
			let current = response;

			try {
				for (
					let attempt = 0;
					current.status === 429 && attempt < retries;
					attempt++
				) {
					const body = await current
						.clone()
						.json()
						.catch(() => undefined);
					const info = ratelimitFromResponse(current, body);
					options.onRatelimit?.(info);

					const delay =
						retryAfterMs(current, body) ??
						fallbackDelay(attempt, baseDelay, maxDelay);
					await sleep(Math.min(delay, maxDelay));

					const request = clones.get(id);
					if (!request) return current;
					current = await clientOptions.fetch(request.clone());
				}

				return current;
			} finally {
				clones.delete(id);
			}
		},
	};
}

export async function retryWithBackoff<T>(
	fn: () => Promise<HttpResult<T>>,
	options: RetryOptions = {},
): Promise<T> {
	const retries = Math.max(1, options.retries ?? DEFAULT_RETRIES);
	const baseDelay = options.baseDelay ?? DEFAULT_BASE_DELAY;
	const maxDelay = options.maxDelay ?? DEFAULT_MAX_DELAY;

	for (let attempt = 0; attempt < retries; attempt++) {
		let result: HttpResult<T>;
		try {
			result = await fn();
		} catch (error) {
			const response = errorResponse(error);
			if (!shouldRetry(response, error) || attempt === retries - 1) {
				throw error;
			}
			if (response?.status === 429) {
				const info = ratelimitFromResponse(response, error);
				options.onRatelimit?.(info);
				await sleep(
					Math.min(
						retryAfterMs(response, error) ??
							fallbackDelay(attempt, baseDelay, maxDelay),
						maxDelay,
					),
				);
			} else {
				await sleep(fallbackDelay(attempt, baseDelay, maxDelay));
			}
			continue;
		}

		const { data, error, response } = result;
		if (!error) return data as T;

		if (!shouldRetry(response, error) || attempt === retries - 1) {
			throw error;
		}

		if (response.status === 429) {
			const info = ratelimitFromResponse(response, error);
			options.onRatelimit?.(info);
			await sleep(
				Math.min(
					retryAfterMs(response, error) ??
						fallbackDelay(attempt, baseDelay, maxDelay),
					maxDelay,
				),
			);
		} else {
			await sleep(fallbackDelay(attempt, baseDelay, maxDelay));
		}
	}

	throw new Error("too many retries");
}
