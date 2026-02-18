import { createMemo, Match, Show, Switch } from "solid-js";
import { getThumbFromId } from "../media/util";
import { getColor } from "../colors";
import icChanText1 from "../assets/channel-text-1.png";
import icChanText2 from "../assets/channel-text-2.png";
import icChanText3 from "../assets/channel-text-3.png";
import icChanText4 from "../assets/channel-text-4.png";
import icChanVoice1 from "../assets/channel-voice-1.png";
import icChanVoice2 from "../assets/channel-voice-2.png";
import icChanForum1 from "../assets/channel-forum-1.png";
import icChanCalendar1 from "../assets/channel-calendar-1.png";
import icChanDocument1 from "../assets/channel-document-1.png";
import icChanWiki1 from "../assets/channel-wiki-1.png";
import icChanNsfw from "../assets/channel-nsfw.png";
import { Channel } from "sdk";
import { useApi } from "../api";
import { cyrb53, LCG } from "../rng";
import { AvatarWithStatus } from "./UserAvatar";

export const ChannelIcon = (
	props: { channel: Channel },
) => {
	const api = useApi();

	const icon = () => {
		const rand = LCG(cyrb53(props.channel.id));
		function rnd<T>(arr: T[]): T {
			return arr[Math.floor(rand() * arr.length)];
		}
		switch (props.channel.type) {
			case "Voice":
				return rnd([icChanVoice1, icChanVoice2]);
			case "Forum":
				return rnd([icChanForum1]);
			case "Calendar":
				return rnd([icChanCalendar1]);
			case "Document":
				return rnd([icChanDocument1]);
			case "Wiki":
				return rnd([icChanWiki1]);
			case "Text":
			default:
				return rnd([icChanText1, icChanText2, icChanText3, icChanText4]);
		}
	};

	const otherUser = createMemo(() => {
		if (props.channel.type === "Dm") {
			const selfId = api.users.cache.get("@self")!.id;
			return props.channel.recipients.find((i) => i.id !== selfId);
		}
		return undefined;
	});

	return (
		<Switch>
			<Match when={props.channel.type === "Dm" && otherUser()}>
				<AvatarWithStatus user={otherUser()} />
			</Match>
			<Match when={props.channel.type === "Gdm"}>
				<ChannelIconGdm id={props.channel.id} icon={props.channel.icon} />
			</Match>
			<Match
				when={["Text", "Voice", "Forum", "Calendar", "Document", "Wiki"]
					.includes(
						props.channel.type,
					)}
			>
				<svg class="icon" viewBox="0 0 64 64">
					<mask id="nsfw">
						<rect
							width={64}
							height={64}
							x={0}
							y={0}
							fill="white"
						/>
						<rect
							rx={4}
							width={32}
							height={32}
							x={32}
							y={0}
							fill="black"
						/>
					</mask>
					<image
						mask={props.channel.nsfw ? "url(#nsfw)" : undefined}
						href={icon()}
					/>
					<Show when={props.channel.nsfw}>
						<image href={icChanNsfw} />
					</Show>
				</svg>
			</Match>
		</Switch>
	);
};

export const ChannelIconGdm = (
	props: { id: string; icon?: string | null; pad?: number },
) => {
	const pad = () => props.pad ?? 4;
	const size = 64;
	const totalSize = () => size + pad() * 2;
	return (
		<svg
			class="avatar"
			viewBox={`0 0 ${totalSize()} ${totalSize()}`}
			role="img"
			style={{ "--pad": `${pad()}px` }}
		>
			<mask id="thread-icon-mask">
				<rect
					rx="6"
					width={size}
					height={size}
					x={pad()}
					y={pad()}
					fill="white"
				/>
			</mask>
			<g mask="url(#thread-icon-mask)">
				<rect
					width={size}
					height={size}
					x={pad()}
					y={pad()}
					fill={props.icon ? "oklch(var(--color-bg3))" : getColor(props.id)}
				/>
				<Show when={props.icon}>
					<image
						preserveAspectRatio="xMidYMid slice"
						width={size}
						height={size}
						x={pad()}
						y={pad()}
						href={getThumbFromId(props.icon!, 64)!}
					/>
				</Show>
			</g>
		</svg>
	);
};
