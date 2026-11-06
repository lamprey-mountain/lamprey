export type PasswordStrength = {
	score: 0 | 1 | 2 | 3 | 4;
	warning: string;
	suggestions: string[];
};

export type PasswordStrengthRequest = {
	id: number;
	password: string;
	inputs: string[];
};

export type PasswordStrengthResponse = {
	id: number;
	strength: PasswordStrength | null;
};

export type PasswordStrengthScoreOptions = {
	/** abort scoring; the returned promise resolves `null` */
	signal?: AbortSignal;
};

export type PasswordStrengthScorer = {
	/**
	 * Score a password in a worker.
	 *
	 * Resolves `null` if the worker failed to start, crashed, timed out, or
	 * scoring was aborted; the caller is responsible for ignoring responses to
	 * stale requests.
	 */
	score(
		password: string,
		inputs: string[],
		options?: PasswordStrengthScoreOptions,
	): Promise<PasswordStrength | null>;

	/** terminate the worker; pending scores resolve with `null` */
	dispose(): void;
};

const MAX_SCORED_PASSWORD_LENGTH = 1024;
const SCORE_TIMEOUT_MS = 5000;

type PendingScore = {
	worker: Worker;
	cancel: () => void;
};

export function createPasswordStrengthScorer(): PasswordStrengthScorer {
	let worker: Worker | undefined;
	let nextRequestId = 0;
	const pending = new Set<PendingScore>();

	function getWorker(): Worker {
		if (worker) return worker;
		const w = new Worker(
			new URL("./password-strength.worker.ts", import.meta.url),
			{ type: "module" },
		);
		worker = w;
		return w;
	}

	function failWorker(w: Worker) {
		if (worker === w) {
			w.terminate();
			worker = undefined;
		}
		for (const score of [...pending]) {
			if (score.worker === w) score.cancel();
		}
	}

	return {
		score(password, inputs, options) {
			if (!password) return Promise.resolve(null);
			if (options?.signal?.aborted) return Promise.resolve(null);

			const id = ++nextRequestId;
			let w: Worker;
			try {
				w = getWorker();
			} catch (e) {
				console.error("failed to start password strength worker:", e);
				return Promise.resolve(null);
			}

			return new Promise((resolve) => {
				let settled = false;
				let timeout: ReturnType<typeof setTimeout> | undefined;

				function settle(value: PasswordStrength | null) {
					if (settled) return;
					settled = true;
					if (timeout !== undefined) clearTimeout(timeout);
					pending.delete(entry);
					w.removeEventListener("message", onMessage);
					w.removeEventListener("error", onError);
					options?.signal?.removeEventListener("abort", cancel);
					resolve(value);
				}

				function cancel() {
					settle(null);
				}

				const entry: PendingScore = { worker: w, cancel };

				function onMessage(e: MessageEvent<PasswordStrengthResponse>) {
					if (e.data.id !== id) return;
					settle(e.data.strength);
				}

				function onError(e: ErrorEvent) {
					console.error("password strength worker failed:", e.message);
					failWorker(w);
				}

				pending.add(entry);
				options?.signal?.addEventListener("abort", cancel, { once: true });
				w.addEventListener("message", onMessage);
				w.addEventListener("error", onError);
				w.postMessage({
					id,
					password: password.slice(0, MAX_SCORED_PASSWORD_LENGTH),
					inputs,
				} satisfies PasswordStrengthRequest);

				timeout = setTimeout(() => {
					console.warn("password strength scoring timed out");
					failWorker(w);
				}, SCORE_TIMEOUT_MS);
			});
		},

		dispose() {
			for (const score of [...pending]) score.cancel();
			pending.clear();
			worker?.terminate();
			worker = undefined;
		},
	};
}
