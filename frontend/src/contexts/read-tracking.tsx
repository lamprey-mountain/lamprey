import { createContext, useContext } from "solid-js";
import type { Api } from "../api.tsx";
import type { ChannelContextT } from "../channelctx";
import type { ReactiveMap } from "@solid-primitives/map";
import type { SetStoreFunction } from "solid-js/store";
import type { Data } from "../context.ts";

export type ReadTrackingContextT = {
	markThreadRead: (
		thread_id: string,
		version_id: string,
		also_local: boolean,
		delay?: boolean,
	) => Promise<void>;
	markCategoryRead: (category_id: string) => Promise<void>;
	markChannelRead: (
		channel_id: string,
		version_id: string,
		also_local: boolean,
		delay: boolean,
	) => Promise<void>;
};

const ReadTrackingContext = createContext<ReadTrackingContextT>();

export function createReadTrackingProvider(
	api: Api,
	channel_contexts: ReactiveMap<string, ChannelContextT>,
	dataUpdate: SetStoreFunction<Data>,
) {
	const markThreadRead = async (
		thread_id: string,
		version_id: string,
		also_local: boolean,
		delay = false,
	) => {
		let ackGraceTimeout: ReturnType<typeof setTimeout> | undefined;
		let ackDebounceTimeout: ReturnType<typeof setTimeout> | undefined;

		if (delay) {
			ackGraceTimeout = setTimeout(() => {
				ackDebounceTimeout = setTimeout(() => {
					markThreadRead(thread_id, version_id, also_local, false);
				}, 800);
			}, 200);
			return;
		}

		const cc = channel_contexts.get(thread_id);

		if (cc) {
			const [_ch, chUpdate] = cc;
			if (also_local) {
				chUpdate("read_marker_id", version_id);
			}
			await api.channels.ack(thread_id, undefined, version_id);
		} else {
			const c = api.channels.cache.get(thread_id);
			if (c) {
				if (also_local) {
					dataUpdate(
						"channels",
						thread_id,
						"read_marker_id",
						c.last_version_id!,
					);
				}
				await api.channels.ack(thread_id, undefined, c.last_version_id!);
			}
		}
	};

	const markCategoryRead = async (category_id: string) => {
		const category = api.channels.cache.get(category_id);
		if (!category || category.type !== "Category") {
			console.warn("not a category");
			return;
		}

		const childChannels = Array.from(api.channels.cache.values()).filter(
			(c) => c.parent_id === category_id && c.room_id === category.room_id,
		);

		const acks = childChannels
			.map((c) => {
				const version_id = c.last_version_id;
				if (!version_id) return null;
				return { channel_id: c.id, version_id };
			})
			.filter((ack): ack is NonNullable<typeof ack> => ack !== null);

		await api.channels.ackBulk(acks);

		for (const child of childChannels) {
			if (child.last_version_id) {
				const cc = channel_contexts.get(child.id);
				if (cc) {
					const [_ch, chUpdate] = cc;
					chUpdate("read_marker_id", child.last_version_id);
				}
			}
		}
	};

	const markChannelRead = async (
		channel_id: string,
		version_id: string,
		also_local: boolean,
		delay: boolean,
	) => {
		if (delay) {
			setTimeout(() => {
				markChannelRead(channel_id, version_id, also_local, false);
			}, 300);
			return;
		}

		const cc = channel_contexts.get(channel_id);
		if (cc) {
			const [_ch, chUpdate] = cc;
			if (also_local) {
				chUpdate("read_marker_id", version_id);
			}
			await api.channels.ack(channel_id, undefined, version_id);
		} else {
			const c = api.channels.cache.get(channel_id);
			if (c) {
				if (also_local) {
					dataUpdate(
						"channels",
						channel_id,
						"read_marker_id",
						c.last_version_id!,
					);
				}
				await api.channels.ack(channel_id, undefined, c.last_version_id!);
			}
		}
	};

	return {
		markThreadRead,
		markCategoryRead,
		markChannelRead,
	};
}

export const ReadTrackingProvider = (
	props: {
		api: Api;
		channel_contexts: ReactiveMap<string, ChannelContextT>;
		dataUpdate: SetStoreFunction<Data>;
		children: import("solid-js").JSX.Element;
	},
) => {
	const value = createReadTrackingProvider(
		props.api,
		props.channel_contexts,
		props.dataUpdate,
	);

	return (
		<ReadTrackingContext.Provider value={value}>
			{props.children}
		</ReadTrackingContext.Provider>
	);
};

export const useReadTracking = () => {
	const context = useContext(ReadTrackingContext);
	if (!context) {
		throw new Error(
			"useReadTracking must be used within a ReadTrackingProvider",
		);
	}
	return context;
};
