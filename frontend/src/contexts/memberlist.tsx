import { createContext, createEffect, type ParentProps, useContext } from "solid-js";
import { useApi2, useMemberList2 } from "@/api";
import type { MemberList } from "@/api";
import type { ReactiveMap } from "@solid-primitives/map";
import { useLocation } from "@solidjs/router";
import { logger } from "../logger";

const memberListCtxLog = logger.for("member_list");

const MemberListContext = createContext<ReactiveMap<string, MemberList>>();

export const MemberListProvider = (props: ParentProps) => {
	const store = useApi2();
	const service = useMemberList2();
	const location = useLocation();

	memberListCtxLog.info("MemberListProvider initialized", {
		service_lists_size: service.lists.size,
	});

	let currentSubscription: string | null = null;
	createEffect(() => {
		const roomIdMatch = location.pathname.match(/\/room\/([^/]+)/);
		if (roomIdMatch) {
			const id = roomIdMatch[1];
			memberListCtxLog.debug("location changed, room match", {
				id,
				currentSubscription,
			});
			if (currentSubscription !== id) {
				currentSubscription = id;
				memberListCtxLog.info("subscribing to room member list", {
					room_id: id,
					ranges: [[0, 199]],
				});
				store.roomMembers.subscribeList(id, [[0, 199]]);
			}
			return;
		}

		const channelIdMatch = location.pathname.match(
			/\/(channel|thread)\/([^/]+)/,
		);
		if (channelIdMatch) {
			const id = channelIdMatch[2];
			memberListCtxLog.debug("location changed, channel match", {
				id,
				currentSubscription,
			});
			if (currentSubscription !== id) {
				currentSubscription = id;
				memberListCtxLog.info("subscribing to thread member list", {
					thread_id: id,
					ranges: [[0, 199]],
				});
				store.threadMembers.subscribeList(id, [[0, 199]]);
			}
			return;
		}

		if (currentSubscription !== null) {
			memberListCtxLog.debug(
				"location changed, no match, clearing subscription",
				{ currentSubscription },
			);
			currentSubscription = null;
			store.client.send({ type: "MemberListSubscribe", ranges: [] });
		}
	});

	memberListCtxLog.debug("MemberListProvider render", {
		lists_count: service.lists.size,
		list_keys: [...service.lists.keys()],
	});

	return (
		<MemberListContext.Provider value={service.lists}>
			{props.children}
		</MemberListContext.Provider>
	);
};

export const useMemberList = () => {
	const ctx = useContext(MemberListContext);
	memberListCtxLog.debug("useMemberList called", {
		has_context: !!ctx,
		lists_count: ctx?.size,
	});
	return ctx!;
};
