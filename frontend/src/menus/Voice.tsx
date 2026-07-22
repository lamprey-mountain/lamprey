import { useNavigate } from "@solidjs/router";
import type { Channel } from "sdk";
import { useApi, useChannels } from "@/api";
import { Checkbox } from "@/atoms/icons";
import { Item, Menu, Separator } from "./Parts.tsx";

export function VoiceMenu(props: { channel: Channel }) {
	const api = useApi();
	const channels = useChannels();
	const nav = useNavigate();

	const channelConfig = () => props.channel.preferences;

	const setFrontend = (frontend: Record<string, any>) => {
		const current = channelConfig() ?? { notifs: {}, frontend: {} };
		const newPrefs = {
			...current,
			frontend: { ...current.frontend, ...frontend },
		};
		channels.cache.set(props.channel.id, {
			...props.channel,
			preferences: newPrefs,
		});
		api.client.http.PUT("/api/v1/preferences/channel/{channel_id}", {
			params: { path: { channel_id: props.channel.id } },
			body: newPrefs,
		});
	};

	const toggle = (key: string) => {
		const current = (channelConfig()?.frontend as any)?.[key] ?? false;
		setFrontend({ [key]: !current });
	};

	return (
		<Menu>
			<Item onClick={() => toggle("grid_view")}>
				<div style="display: flex; align-items: center; gap: 8px">
					<Checkbox
						checked={(channelConfig()?.frontend as any)?.grid_view ?? false}
						seed={`${props.channel.id}-voice-view-grid`}
					/>
					Grid view
				</div>
			</Item>
			<Item onClick={() => toggle("show_people_without_video")}>
				<div style="display: flex; align-items: center; gap: 8px">
					<Checkbox
						checked={
							(channelConfig()?.frontend as any)?.show_people_without_video ??
							true
						}
						seed={`${props.channel.id}-voice-show-videoless`}
					/>
					Show people without video
				</div>
			</Item>
			<Separator />
			<Item onClick={() => nav("/settings/voice")}>Voice settings</Item>
		</Menu>
	);
}
