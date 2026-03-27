// Minimal API exports for new service pattern
// Use useApi2() and service-specific hooks instead.

import { createContext, useContext } from "solid-js";
import type { Resource } from "solid-js";
import type { Pagination } from "sdk";
export { RootStore } from "./api/core/Store.ts";
import type { RootStore } from "./api/core/Store.ts";

// Re-export service types
export type { InboxService } from "./api/services/InboxService";
export type { DocumentsService } from "./api/services/DocumentsService";
export type { RoomAnalyticsService } from "./api/services/RoomAnalyticsService";
export type { ChannelsService } from "./api/services/ChannelsService";
export type { MessagesService } from "./api/services/MessagesService";
export type { NotificationService } from "./api/services/NotificationService";
export type { PreferencesService } from "./api/services/PreferencesService";

// Re-export other types
export type { MemberList } from "./api/services/MemberListService";
export type { MessageListAnchor } from "./api/services/MessagesService";
export type { RevisionContent } from "./api/services/DocumentsService";
export type { Aggregation } from "./api/services/RoomAnalyticsService";

// Backwards compatibility type - maps old Api property names to RootStore
export type Api = RootStore;

const ApiContext = createContext<never>();
export const RootStoreContext = createContext<RootStore>();

export function useApi2() {
	return useContext(RootStoreContext)!;
}

// Aliases for backwards compatibility
export const useApi = useApi2;

export function useRooms2() {
	return useApi2().rooms;
}

export function useChannels2() {
	return useApi2().channels;
}

export function useUsers2() {
	return useApi2().users;
}

export function useRoles2() {
	return useApi2().roles;
}

export function useSessions2() {
	return useApi2().sessions;
}

export function useMessages2() {
	return useApi2().messages;
}

export function useRoomMembers2() {
	return useApi2().roomMembers;
}

export function useMemberList2() {
	return useApi2().memberLists;
}

export function useThreadMembers2() {
	return useApi2().threadMembers;
}

export function useInvites2() {
	return useApi2().invites;
}

export function useAuth2() {
	return useApi2().auth;
}

export function useDms2() {
	return useApi2().dms;
}

export function useEmoji2() {
	return useApi2().emoji;
}

export function usePush2() {
	return useApi2().push;
}

export function useReactions2() {
	return useApi2().reactions;
}

export function useRoomAnalytics2() {
	return useApi2().roomAnalytics;
}

export function useRoomBans2() {
	return useApi2().roomBans;
}

export function useTags2() {
	return useApi2().tags;
}

export function useThreads2() {
	return useApi2().threads;
}

export function useWebhooks2() {
	return useApi2().webhooks;
}

export function useAuditLog2() {
	return useApi2().auditLog;
}

export function useInbox2() {
	return useApi2().inbox;
}

export function usePreferences2() {
	return useApi2().preferences;
}

export type Listing<T> = {
	resource: Resource<Pagination<T>>;
	pagination: Pagination<T> | null;
	mutate: (value: Pagination<T>) => void;
	refetch: () => void;
	prom: Promise<unknown> | null;
};
