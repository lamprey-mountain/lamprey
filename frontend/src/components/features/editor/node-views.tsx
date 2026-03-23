import { getOwner, runWithOwner, VoidComponent } from "solid-js";
import { render } from "solid-js/web";
import { getEmojiUrl } from "../../../media/util.tsx";
import { type Api, useChannels2 } from "../../../api.tsx";
import { getTwemoji, getTwemojiUrl } from "../../../emoji.ts";

export const createNodeViews = () => {
	const owner = getOwner();

	return function <T extends Record<string, any>>(
		propsFn: (node: any) => T,
		Component: VoidComponent<T>,
	) {
		return (node: any) => {
			const dom = document.createElement("span");
			dom.classList.add("node-view-wrapper");

			const dispose = render(
				() => runWithOwner(owner, () => <Component {...propsFn(node)} />),
				dom,
			);

			return {
				dom,
				destroy: () => dispose(),
			};
		};
	};
};

export const createEditorNodeViews = (
	api: Api,
	channels2: ReturnType<typeof useChannels2>,
	opts?: {
		currentChannelId?: () => string;
	},
) => {
	const nv = createNodeViews();

	return () => ({
		mention: nv(
			(n) => ({ id: n.attrs.user, name: n.attrs.name }),
			(props) => {
				const getUserId = () => props.id;
				if (opts?.currentChannelId) {
					const channel = channels2.use(() => opts.currentChannelId!());
					const user = api.users.fetch(getUserId);
					const roomMember = api.room_members.fetch(
						() => channel()?.room_id!,
						getUserId,
					);
					const name = () => {
						const id = getUserId();
						if (!id) return "..."; // Placeholder while loading/missing
						return roomMember()?.override_name ?? user()?.name ?? id;
					};
					return <span class="mention mention-user">@{name()}</span>;
				} else {
					const user = api.users.fetch(getUserId);
					return (
						<span class="mention mention-user">
							@{user()?.name ?? getUserId() ?? "..."}
						</span>
					);
				}
			},
		),
		mentionChannel: nv(
			(n) => ({ id: n.attrs.channel, name: n.attrs.name }),
			(props) => {
				const getChannelId = () => props.id;
				const channel = channels2.use(getChannelId);
				const name = () => channel()?.name ?? getChannelId() ?? "...";
				return <span class="mention mention-channel">#{name()}</span>;
			},
		),
		mentionRole: nv(
			(n) => ({ id: n.attrs.role, name: n.attrs.name }),
			(props) => {
				const getRoleId = () => props.id;
				const role = () => api.roles.cache.get(getRoleId());
				const name = () => role()?.name ?? getRoleId() ?? "...";
				return <span class="mention mention-role">@{name()}</span>;
			},
		),
		mentionEveryone: nv(
			() => ({}),
			() => {
				return <span class="mention mention-everyone">@everyone</span>;
			},
		),
		emojiCustom: nv(
			(n) => ({ id: n.attrs.id, name: n.attrs.name }),
			(props) => {
				const url = getEmojiUrl(props.id);
				return (
					<img
						class="emoji"
						src={url}
						alt={`:${props.name ?? ""}:`}
						title={`:${props.name ?? ""}:`}
					/>
				);
			},
		),
		emojiUnicode: nv(
			(n) => ({ char: n.attrs.char }),
			(props) => {
				const emojiSrc = getTwemojiUrl(props.char);
				if (emojiSrc) {
					return (
						<img
							src={emojiSrc}
							alt={props.char}
							title={props.char}
							class="emoji"
						/>
					);
				}
				return <span>{props.char}</span>;
			},
		),
	});
};
