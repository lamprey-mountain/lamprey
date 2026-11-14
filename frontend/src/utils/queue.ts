import { logger } from "./logger";

const log = logger.for("queue");

type QueueState = "idle" | "active" | "pending";

/**
 * Serial task queue. Tasks run one at a time, in push order, except for tasks
 * pushed with `pushPriority`, which are drained first (FIFO among themselves).
 */
export class Queue<T> {
	private tasks: T[] = [];
	private priority: T[] = [];
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
		this.schedule();
	}

	pushPriority(...tasks: T[]) {
		this.priority.push(...tasks);
		this.schedule();
	}

	some(predicate: (task: T) => boolean) {
		return this.priority.some(predicate) || this.tasks.some(predicate);
	}

	private schedule() {
		if (this.state === "idle") {
			this.state = "pending";
			queueMicrotask(() => this.drain());
		}
	}

	cancel(reason?: string) {
		this.abortCtl.abort(reason);
		this.tasks = [];
		this.priority = [];
	}

	async drain() {
		if (this.state === "active") return;
		this.state = "active";
		this.abortCtl = new AbortController();
		const s = this.abortCtl.signal;

		try {
			let next: T | undefined;
			while ((next = this.priority.shift() ?? this.tasks.shift())) {
				if (s.aborted) break;

				// a throwing task must not wedge the queue: keep draining
				try {
					await this.handler(next, s);
				} catch (err) {
					log.error("task failed", { task: next, error: err });
				}
			}
		} finally {
			this.state = "idle";
		}
	}

	get active() {
		return this.state !== "idle";
	}
}
