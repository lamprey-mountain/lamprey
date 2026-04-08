import type { Pagination } from "sdk";
import type { Resource } from "solid-js";
import { createContext, useContext } from "solid-js";

export { RootStore } from "./api/core/Store.ts";

import type { RootStore } from "./api/core/Store.ts";

export type { ChannelsService } from "./api/services/ChannelsService";
export type {
	DocumentsService,
	RevisionContent,
} from "./api/services/DocumentsService";
// Re-export service types
export type { InboxService } from "./api/services/InboxService";
// Re-export other types
export type { MemberList } from "./api/services/MemberListService";
export type {
	MessageListAnchor,
	MessagesService,
} from "./api/services/MessagesService";
export type { NotificationService } from "./api/services/NotificationService";
export type { PreferencesService } from "./api/services/PreferencesService";
export type {
	Aggregation,
	RoomAnalyticsService,
} from "./api/services/RoomAnalyticsService";

// Backwards compatibility type - maps old Api property names to RootStore
export type Api = RootStore;

export const RootStoreContext = createContext<RootStore>();

export function useApi() {
	const ctx = useContext(RootStoreContext);
	if (!ctx) {
		throw new Error("useApi must be used within a RootStoreContext.Provider");
	}
	return ctx;
}

export function useRooms() {
	return useApi().rooms;
}

export function useChannels() {
	return useApi().channels;
}

export function useUsers() {
	return useApi().users;
}

export function useRoles() {
	return useApi().roles;
}

export function useSessions() {
	return useApi().sessions;
}

export function useMessages() {
	return useApi().messages;
}

export function useRoomMembers() {
	return useApi().roomMembers;
}

export function useMemberList() {
	return useApi().memberLists;
}

export function useThreadMembers() {
	return useApi().threadMembers;
}

export function useInvites() {
	return useApi().invites;
}

export function useAuth() {
	return useApi().auth;
}

export function useDms() {
	return useApi().dms;
}

export function useEmoji() {
	return useApi().emoji;
}

export function usePush() {
	return useApi().push;
}

export function useReactions() {
	return useApi().reactions;
}

export function useRoomAnalytics() {
	return useApi().roomAnalytics;
}

export function useRoomBans() {
	return useApi().roomBans;
}

export function useTags() {
	return useApi().tags;
}

export function useThreads() {
	return useApi().threads;
}

export function useWebhooks() {
	return useApi().webhooks;
}

export function useAuditLog() {
	return useApi().auditLog;
}

export function useInbox() {
	return useApi().inbox;
}

export function usePreferences() {
	return useApi().preferences;
}

export type Listing<T> = {
	resource: Resource<Pagination<T>>;
	pagination: Pagination<T> | null;
	mutate: (value: Pagination<T>) => void;
	refetch: () => void;
	prom: Promise<unknown> | null;
};
