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

export type PasswordStrengthScorer = {
	/**
	 * Score a password in a worker.
	 *
	 * Resolves `null` if the worker failed to start or crashed; the caller is
	 * responsible for ignoring responses to stale requests.
	 */
	score(password: string, inputs: string[]): Promise<PasswordStrength | null>;

	/** terminate the worker; pending scores never resolve */
	dispose(): void;
};

export function createPasswordStrengthScorer(): PasswordStrengthScorer {
	let worker: Worker | undefined;
	let nextRequestId = 0;

	function getWorker(): Worker {
		if (worker) return worker;
		const w = new Worker(
			new URL("./password-strength.worker.ts", import.meta.url),
			{ type: "module" },
		);
		worker = w;
		return w;
	}

	return {
		score(password, inputs) {
			if (!password) return Promise.resolve(null);

			const id = ++nextRequestId;
			let w: Worker;
			try {
				w = getWorker();
			} catch (e) {
				console.error("failed to start password strength worker:", e);
				return Promise.resolve(null);
			}

			return new Promise((resolve) => {
				function cleanup() {
					w.removeEventListener("message", onMessage);
					w.removeEventListener("error", onError);
				}

				function onMessage(e: MessageEvent<PasswordStrengthResponse>) {
					if (e.data.id !== id) return;
					cleanup();
					resolve(e.data.strength);
				}

				function onError(e: ErrorEvent) {
					console.error("password strength worker failed:", e.message);
					cleanup();
					if (worker === w) {
						w.terminate();
						worker = undefined;
					}
					resolve(null);
				}

				w.addEventListener("message", onMessage);
				w.addEventListener("error", onError);
				w.postMessage({
					id,
					password,
					inputs,
				} satisfies PasswordStrengthRequest);
			});
		},

		dispose() {
			worker?.terminate();
			worker = undefined;
		},
	};
}
