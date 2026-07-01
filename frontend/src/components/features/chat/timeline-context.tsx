import {
	createGlobalEmitter,
	GlobalEmitter,
} from "@solid-primitives/event-bus";
import { createContext, useContext, ParentProps } from "solid-js";
import { createStore, SetStoreFunction } from "solid-js/store";
import { MessageListAnchor } from "@/api/services/MessagesService";
import { MessageRange } from "@/api/services/MessagesService";
import { TimelineItemT } from "./Messages";
import { useChannel } from "@/contexts/mod";
import { ChannelT } from "@/types";

export type TimelineController = {
	jumpToBottom(smooth?: boolean): void;
	jumpToTop(smooth?: boolean): void;
	jumpToMessage(
		message_id: string,
		smooth?: boolean,
		highlight?: boolean,
	): void;
	scrollBy(px: number, smooth?: boolean): void;
	ackMessage(message_id: string): void;

	events: GlobalEmitter<TimelineEvents>;
	commands: GlobalEmitter<TimelineCommands>;
};

export type TimelineEvents = {
	/** scroll position updated */
	scrollPosition: number;

	/** scrolled to top of channel */
	scrollTop: void;

	/** scrolled to bottom of channel */
	scrollBottom: void;
};

export type TimelineCommands = {
	scrollBy: { px: number; smooth: boolean };
	jumpToBottom: { smooth: boolean };
	jumpToTop: { smooth: boolean };
	jumpToMessage: { message_id: string; smooth: boolean; highlight: boolean };
	ackMessage: { message_id: string };
};

export type TimelineState = {
	messages: MessageRange | null;
	anchor: MessageListAnchor;
	loading: boolean;
	highlight: string | null;
	scroll_pos: number;
	has_forward: boolean;
	items: TimelineItemT[];
	last_read_message_id?: string;

	controller: TimelineController;
	events: GlobalEmitter<TimelineEvents>;
	commands: GlobalEmitter<TimelineCommands>;
};

export type TimelineStore = [TimelineState, SetStoreFunction<TimelineState>];

export const TimelineContext = createContext<TimelineStore>();

export type TimelineProviderProps = ParentProps & {
	channel: ChannelT;
};

export const TimelineProvider = (props: TimelineProviderProps) => {
	const [chanState, updateChanState] = useChannel();
	let store = chanState.timelineStore;

	if (!store) {
		const getInitialAnchor = (): MessageListAnchor => {
			const readMarker = props.channel.last_read_id;
			const hasReadMarker =
				readMarker && readMarker !== props.channel.last_version_id;
			if (hasReadMarker) {
				return { type: "context", limit: 50, message_id: readMarker };
			} else {
				return { type: "backwards", limit: 50 };
			}
		};

		store = createStore<TimelineState>({
			messages: null,
			loading: false,
			highlight: null,
			scroll_pos: 0,
			has_forward: false,
			controller: chanState.timeline,
			items: [],
			anchor: getInitialAnchor(),
			last_read_message_id: props.channel.last_read_id ?? undefined,
			events: chanState.timeline.events,
			commands: chanState.timeline.commands,
		});

		updateChanState("timelineStore", store);
	}

	return (
		<TimelineContext.Provider value={store}>
			{props.children}
		</TimelineContext.Provider>
	);
};

export const useTimeline = (): TimelineStore => {
	const ctx = useContext(TimelineContext);
	if (!ctx) {
		throw new Error(
			"useTimeline must be used within a TimelineContext.Provider",
		);
	}
	return ctx;
};

export const createTimelineController = (): TimelineController => {
	const events = createGlobalEmitter<TimelineEvents>();
	const commands = createGlobalEmitter<TimelineCommands>();

	return {
		jumpToBottom(smooth = false) {
			commands.emit("jumpToBottom", { smooth });
		},
		jumpToTop(smooth = false) {
			commands.emit("jumpToTop", { smooth });
		},
		jumpToMessage(message_id, smooth = false, highlight = false) {
			commands.emit("jumpToMessage", { message_id, smooth, highlight });
		},
		scrollBy(px: number, smooth = false) {
			commands.emit("scrollBy", { px, smooth });
		},
		ackMessage(message_id: string) {
			commands.emit("ackMessage", { message_id });
		},

		events,
		commands,
	};
};
