export class Queue<T> {
	private tasks: T[] = [];
	private active = false;
	private abortCtl = new AbortController();

	constructor(
		private handler: (
			task: T,
			abortSignal: AbortSignal,
		) => Promise<void> | void,
	) {}

	push(...tasks: T[]) {
		this.tasks.push(...tasks);
		this.drain();
	}

	cancel(reason?: string) {
		this.abortCtl.abort(reason);
		this.tasks = [];
	}

	async drain() {
		if (this.active) return;
		this.active = true;
		this.abortCtl = new AbortController();
		const s = this.abortCtl.signal;

		let next;
		while ((next = this.tasks.shift())) {
			if (s.aborted) break;
			await this.handler(next, s);
		}

		this.active = false;
	}
}
