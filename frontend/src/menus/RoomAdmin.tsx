import type { api } from "sdk";
import { For, Show } from "solid-js";
import { useApi, useRooms } from "@/api";
import { useModals } from "@/contexts/modal";
import { Item, Menu, Separator, Submenu } from "./Parts.tsx";

type RoomAdminMenuProps = {
	room_id: string;
};

type Feature = {
	id: api["RoomFeature"];
	label: string;
	description: string;
};

// must mirror the backend `RoomFeature` enum
const FEATURES: Feature[] = [
	{ id: "Scripts", label: "scripts", description: "script channels" },
	{ id: "Automod", label: "automod", description: "experimental automod" },
	{
		id: "Documents",
		label: "documents",
		description: "experimental documents and wikis",
	},
	{ id: "Vanity", label: "vanity", description: "vanity invite codes" },
	{
		id: "Llm",
		label: "llm",
		description: "llm-based thread titles and summaries",
	},
];

export function RoomAdminMenu(props: RoomAdminMenuProps) {
	const api2 = useApi();
	const rooms2 = useRooms();
	const room = rooms2.use(() => props.room_id);
	const [, modalCtl] = useModals();

	const copyRoomId = () => navigator.clipboard.writeText(props.room_id);
	const logToConsole = () => console.log(JSON.parse(JSON.stringify(room())));

	const hasFeature = (feature: api["RoomFeature"]) =>
		room()?.features.includes(feature) ?? false;

	const enableFeature = async (feature: api["RoomFeature"]) => {
		const { data } = await api2.client.http.PUT(
			"/api/v1/room/{room_id}/feature/{feature}",
			{ params: { path: { room_id: props.room_id, feature } } },
		);
		if (data) rooms2.upsert(data);
	};

	const disableFeature = async (feature: api["RoomFeature"]) => {
		const { data } = await api2.client.http.DELETE(
			"/api/v1/room/{room_id}/feature/{feature}",
			{ params: { path: { room_id: props.room_id, feature } } },
		);
		if (data) rooms2.upsert(data);
	};

	const toggleFeature = (feature: api["RoomFeature"]) =>
		hasFeature(feature) ? disableFeature(feature) : enableFeature(feature);

	const quarantine = () => {
		modalCtl.confirm(
			"Are you sure you want to quarantine this room? all members will be read-only until it is unquarantined.",
			async (confirmed) => {
				if (!confirmed) return;
				const { data } = await api2.client.http.POST(
					"/api/v1/room/{room_id}/quarantine",
					{ params: { path: { room_id: props.room_id } } },
				);
				if (data) rooms2.upsert(data);
			},
		);
	};

	const unquarantine = () => {
		modalCtl.confirm(
			"Are you sure you want to unquarantine this room?",
			async (confirmed) => {
				if (!confirmed) return;
				const { data } = await api2.client.http.DELETE(
					"/api/v1/room/{room_id}/quarantine",
					{ params: { path: { room_id: props.room_id } } },
				);
				if (data) rooms2.upsert(data);
			},
		);
	};

	const deleteRoom = () => {
		modalCtl.confirm(
			"Are you sure you want to delete this room? This action can be undone with undelete.",
			async (confirmed) => {
				if (!confirmed) return;
				await api2.client.http.DELETE("/api/v1/room/{room_id}", {
					params: { path: { room_id: props.room_id } },
				});
			},
		);
	};

	const undeleteRoom = () => {
		modalCtl.confirm(
			"Are you sure you want to undelete this room?",
			async (confirmed) => {
				if (!confirmed) return;
				await api2.client.http.POST("/api/v1/room/{room_id}/undelete", {
					params: { path: { room_id: props.room_id } },
				});
			},
		);
	};

	// TODO: actually transfer ownership
	const transferOwnership = () => {
		modalCtl.prompt("new owner user id", (user_id) => {
			if (!user_id) return;
			console.log("[room] TODO: transfer ownership", {
				room_id: props.room_id,
				user_id,
			});
		});
	};

	return (
		<Menu>
			<Submenu content="features">
				<For each={FEATURES}>
					{(feature) => (
						<Item
							onClick={() => toggleFeature(feature.id)}
							classList={{ selected: hasFeature(feature.id) }}
						>
							<div>{feature.label}</div>
							<div class="dim">{feature.description}</div>
						</Item>
					)}
				</For>
			</Submenu>
			<Item onClick={transferOwnership}>transfer ownership</Item>
			<Separator />
			<Show
				when={room()?.quarantined}
				fallback={
					<Item onClick={quarantine} color="danger">
						quarantine room
					</Item>
				}
			>
				<Item onClick={unquarantine}>unquarantine room</Item>
			</Show>
			<Show
				when={room()?.deleted_at}
				fallback={
					<Item onClick={deleteRoom} color="danger">
						delete room
					</Item>
				}
			>
				<Item onClick={undeleteRoom}>undelete room</Item>
			</Show>
			<Separator />
			<Item onClick={copyRoomId}>copy room id</Item>
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}
