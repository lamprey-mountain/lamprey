import {
	createGlobalEmitter,
	GlobalEmitter,
} from "@solid-primitives/event-bus";
import {
	createContext,
	useContext,
	ParentProps,
	Signal,
	createSignal,
} from "solid-js";
import { MessageListAnchor } from "@/api/services/MessagesService";
import { MessageRange } from "@/api/services/MessagesService";
import { useChannel } from "@/contexts/mod";
import { ChannelT } from "@/types";
import { TimelineItemT2 } from "./util";
import { unwrap } from "solid-js/store";

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
	loading: boolean; // NOTE: unnecessary with queue system?
	highlight: string | null;
	scrollTop: number;
	items: TimelineItemT2[];
	readMarkerId: string | null;

	controller: TimelineController;
	events: GlobalEmitter<TimelineEvents>;
	commands: GlobalEmitter<TimelineCommands>;
};

export const TimelineContext = createContext<TimelineState>();

export type TimelineProviderProps = ParentProps & {
	channel: ChannelT;
};

export const TimelineProvider = (props: TimelineProviderProps) => {
	const [chanState, updateChanState] = useChannel();
	let state = unwrap(chanState.timelineState);

	if (!state) {
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

		state = {
			messages: null,
			loading: false,
			highlight: null,
			scrollTop: 0,
			controller: chanState.timeline,
			items: [{ type: "skeletons", key: "skeletons-top" }],
			anchor: getInitialAnchor(),
			readMarkerId: props.channel.last_read_id ?? null,
			events: chanState.timeline.events,
			commands: chanState.timeline.commands,
		};

		updateChanState("timelineState", state);
	}

	return (
		<TimelineContext.Provider value={state}>
			{props.children}
		</TimelineContext.Provider>
	);
};

export const useTimeline = (): TimelineState => {
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
