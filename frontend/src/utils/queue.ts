type QueueState = "idle" | "active" | "pending";

export class Queue<T> {
	private tasks: T[] = [];
	private abortCtl = new AbortController();
	private state: QueueState = "idle";

	constructor(
		private handler: (
			task: T,
			abortSignal: AbortSignal,
		) => Promise<void> | void,
	) {}

	push(...tasks: T[]) {
		this.tasks.push(...tasks);

		if (this.state === "idle") {
			this.state = "pending";
			queueMicrotask(() => this.drain());
		}
	}

	cancel(reason?: string) {
		this.abortCtl.abort(reason);
		this.tasks = [];
	}

	async drain() {
		if (this.state === "active") return;
		this.state = "active";
		this.abortCtl = new AbortController();
		const s = this.abortCtl.signal;

		let next;
		while ((next = this.tasks.shift())) {
			if (s.aborted) break;
			await this.handler(next, s);
		}

		this.state = "idle";
	}

	get active() {
		return this.state !== "idle";
	}
}
