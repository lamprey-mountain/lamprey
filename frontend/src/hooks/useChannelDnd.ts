import { createEffect, createSignal } from "solid-js";
import { type ChannelsService, useApi, useChannels } from "@/api";
import type { ChannelT } from "@/types";
import { logger } from "@/utils/logger";

const log = logger.for("channel_nav/dnd");

type DndStateNone = { type: "none" };

type DndStateChannel = {
	type: "channel";
	id: string;
	targetId?: string;
	place?: "before" | "after" | "inside-category";
};

type DndStateCategory = {
	type: "category";
	id: string;
	targetId?: string;
	place?: "before" | "after";
};

type DndStateThread = {
	type: "thread";
	id: string;
	parentId?: string;
};

type DndStateVoice = {
	type: "voice";
	userId: string;
	channelId: string;
	targetChannelId?: string;
};

type DndState =
	| DndStateNone
	| DndStateChannel
	| DndStateCategory
	| DndStateThread
	| DndStateVoice;

// for styling
export type DndPlacement =
	| "before"
	| "after"
	| "inside-thread"
	| "inside-category"
	| "inside-voice";

const isVoice = (chan: ChannelT): boolean => {
	return chan.type === "Voice" || chan.type === "Broadcast";
};

const isThread = (chan: ChannelT): boolean => {
	return (
		chan.type === "ThreadPublic" ||
		chan.type === "ThreadPrivate" ||
		chan.type === "ThreadForum2"
	);
};

/** return if a thread channel can be in another channel */
const canThreadBeIn = (child: ChannelT, parent: ChannelT): boolean => {
	if (child.type === "ThreadPublic" || child.type === "ThreadPrivate") {
		return ["Text", "Announcement", "Forum"].includes(parent.type);
	}

	if (child.type === "ThreadForum2") {
		return parent.type === "Forum2";
	}

	if (child.type === "Document") {
		return parent.type === "Wiki";
	}

	return false;
};

const calculateDndState = (
	channels: ChannelsService,
	e: DragEvent,
	cd: ChannelT,
	ct: ChannelT,
): Partial<DndStateChannel | DndStateCategory | DndStateThread> | null => {
	// move threads between channels
	if (canThreadBeIn(cd, ct)) {
		return { type: "thread", parentId: ct.id };
	}

	// use target's parent if target is a thread
	if (isThread(ct) && ct.parent_id) {
		const parent = channels.cache.get(ct.parent_id);
		if (parent) return calculateDndState(channels, e, cd, parent);
	}

	// reorder categories
	if (cd.type === "Category") {
		if (ct.type === "Category") {
			return { type: "category", targetId: ct.id, place: "after" };
		} else if (ct.parent_id) {
			const parent = channels.cache.get(ct.parent_id);
			if (parent) return calculateDndState(channels, e, cd, parent);
		} else {
			const allCategories = [...channels.listByRoom(cd.room_id!)].filter(
				(c) => c.type === "Category",
			);
			allCategories.sort((a, b) => (a.position ?? 0) - (b.position ?? 0));

			const first = allCategories[0];
			if (first) {
				return { type: "category", targetId: first.id, place: "before" };
			}
		}
	}

	// dragging channel into a category
	if (ct.type === "Category") {
		return { type: "channel", targetId: ct.id, place: "inside-category" };
	}

	// reorder channels
	const el = e.currentTarget as HTMLElement;
	const rect = el.getBoundingClientRect();
	const after = e.clientY > rect.top + rect.height / 2;
	return {
		type: "channel",
		targetId: ct.id,
		place: after ? "after" : "before",
	};
};

export const useChannelDnd = () => {
	const api = useApi();
	const channels = useChannels();

	const [state, setState] = createSignal<DndState>({ type: "none" });

	const handleDrop = (_e: DragEvent) => {
		const s = state();
		if (s.type === "none") return;

		if (s.type === "voice") {
			if (!s.targetChannelId) return;
			const ct = channels.cache.get(s.targetChannelId);
			if (!ct || !isVoice(ct)) return;

			log.info("drop", "move voice");

			api.client.http.POST("/api/v1/voice/{channel_id}/member/{user_id}/move", {
				params: {
					path: {
						channel_id: s.channelId,
						user_id: s.userId,
					},
				},
				body: {
					target_id: s.targetChannelId,
				},
			});
		} else {
			const cd = channels.cache.get(s.id);
			if (!cd) return;

			let targetId: string | undefined;
			let place: string | undefined;

			if (s.type === "thread") {
				targetId = s.parentId;
				place = "inside-thread";
			} else if (s.type === "category") {
				targetId = s.targetId;
				place = s.place;
			} else if (s.type === "channel") {
				targetId = s.targetId;
				place = s.place;
			}

			if (!targetId || !place) return;
			const ct = channels.cache.get(targetId);
			const roomId = ct?.room_id;
			if (!ct || !roomId) return;

			// reparenting
			if (place === "inside-thread" || place === "inside-category") {
				log.info("drop", "reparent thread");
				channels.update(cd.id, { parent_id: ct.id });
				return;
			}

			if (cd.type === "Category") {
				// reordering categories
				const allCategories = [...channels.listByRoom(roomId)].filter(
					(c) => c.type === "Category",
				);
				allCategories.sort((a, b) => (a.position ?? 0) - (b.position ?? 0));

				const fromIndex = allCategories.findIndex((c) => c.id === cd.id);
				if (fromIndex !== -1) allCategories.splice(fromIndex, 1);

				let toIndex = allCategories.findIndex((c) => c.id === ct.id);
				if (place === "after") toIndex++;

				allCategories.splice(toIndex, 0, cd);

				const body = allCategories.map((c, i) => ({
					id: c.id,
					parent_id: null,
					position: i,
				}));

				log.info("drop", "reorder categories", body);

				api.client.http.PATCH("/api/v1/room/{room_id}/channel", {
					params: { path: { room_id: roomId } },
					body: { channels: body },
				});
			} else {
				// reordering channels
				const siblings = [...channels.listByRoom(roomId)].filter(
					(c) => c.parent_id === ct.parent_id,
				);
				siblings.sort((a, b) => (a.position ?? 0) - (b.position ?? 0));

				const fromIndex = siblings.findIndex((c) => c.id === cd.id);
				if (fromIndex !== -1) siblings.splice(fromIndex, 1);

				let toIndex = siblings.findIndex((c) => c.id === ct.id);
				if (place === "after") toIndex++;

				siblings.splice(toIndex, 0, cd);

				const body = siblings.map((c, i) => ({
					id: c.id,
					parent_id: ct.parent_id ?? null,
					position: i,
				}));

				log.info("drop", "reorder channels", body);

				api.client.http.PATCH("/api/v1/room/{room_id}/channel", {
					params: { path: { room_id: roomId } },
					body: { channels: body },
				});
			}
		}
	};

	const handleStart = (e: DragEvent) => {
		const el = e.currentTarget as HTMLElement;
		const isVoiceDrag = el.classList.contains("voice-participant");
		const channelId = el.dataset.channelId;
		const userId = el.dataset.userId;

		if (isVoiceDrag) {
			if (!channelId || !userId) return;
			e.dataTransfer?.setData("text/plain", userId);
			setState({
				type: "voice",
				userId,
				channelId,
				targetChannelId: undefined,
			});
		} else {
			if (!channelId) return;
			e.dataTransfer?.setData("text/plain", channelId);
			const cd = channels.cache.get(channelId);
			if (cd?.type === "Category") {
				setState({
					type: "category",
					id: channelId,
					targetId: undefined,
					place: undefined,
				});
			} else if (cd && isThread(cd)) {
				setState({ type: "thread", id: channelId, parentId: undefined });
			} else {
				setState({
					type: "channel",
					id: channelId,
					targetId: undefined,
					place: undefined,
				});
			}
		}
	};

	const handleOver = (e: DragEvent) => {
		const s = state();
		if (s.type === "none") return;

		const el = e.currentTarget as HTMLElement;
		const channelId = el.dataset.channelId;
		if (!channelId) return;

		const ct = channels.cache.get(channelId);
		if (!ct) return;

		if (s.type === "voice") {
			if (isVoice(ct)) {
				setState({ ...s, targetChannelId: channelId });
			}
		} else if (
			s.type === "channel" ||
			s.type === "category" ||
			s.type === "thread"
		) {
			const id = s.id;
			const cd = channels.cache.get(id);
			if (!cd) return;

			const update = calculateDndState(channels, e, cd, ct);
			if (update) {
				setState({ ...s, ...update } as any);
			}
		}
	};

	const handleEnd = (_e: DragEvent) => {
		setState({ type: "none" });
	};

	const handle = (e: DragEvent) => {
		e.stopPropagation();

		if (e.dataTransfer) {
			e.dataTransfer.effectAllowed = "move";
		}

		log.debug("handle", e.type, e);

		switch (e.type) {
			case "dragstart":
				handleStart(e);
				break;
			case "dragover":
				e.preventDefault();
				handleOver(e);
				break;
			case "dragend":
				e.preventDefault();
				handleEnd(e);
				break;
			case "drop":
				e.preventDefault();
				handleDrop(e);
				break;
		}
	};

	const placement = (id: string) => {
		const s = state();
		if (s.type === "none") return undefined;
		if (s.type === "voice" && s.targetChannelId === id) return "inside-voice";
		if (s.type === "thread" && s.parentId === id) return "inside-thread";
		if (s.type === "category" && s.targetId === id) return s.place;
		if (s.type === "channel" && s.targetId === id) return s.place;
		return undefined;
	};

	createEffect(() => {
		log.info("dnd state", JSON.parse(JSON.stringify(state())));
	});

	return {
		handle,
		placement,
	};
};
