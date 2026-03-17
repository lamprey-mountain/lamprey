import { getOwner, runWithOwner } from "solid-js";
import { render } from "solid-js/web";
import { getEmojiUrl } from "../media/util.tsx";
import { type Api } from "../api.tsx";

export const createNodeViews = () => {
	const owner = getOwner();

	return (
		Component: any,
		propsFn: (node: any) => any,
	) => {
		return (node: any) => {
			const dom = document.createElement("span");
			dom.classList.add("mention-wrapper");

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
	opts?: {
		currentChannelId?: () => string;
	},
) => {
	const nv = createNodeViews();

	return () => ({
		mention: nv(
			(node: any) => {
				const userId = node.attrs.user;
				if (opts?.currentChannelId) {
					const channel = api.channels.fetch(() => opts.currentChannelId!());
					const user = api.users.fetch(() => userId);
					const roomMember = api.room_members.fetch(
						() => channel()?.room_id!,
						() => userId,
					);
					const name = () =>
						roomMember()?.override_name ?? user()?.name ?? userId;
					return <span class="mention-user">@{name()}</span>;
				} else {
					const user = api.users.fetch(() => userId);
					return <span class="mention-user">@{user()?.name ?? userId}</span>;
				}
			},
			(n) => ({ id: n.attrs.user }),
		),
		mentionChannel: nv(
			(node: any) => {
				const channelId = node.attrs.channel;
				const channel = api.channels.fetch(() => channelId);
				return (
					<span class="mention-channel">#{channel()?.name ?? channelId}</span>
				);
			},
			(n) => ({ id: n.attrs.channel }),
		),
		mentionRole: nv(
			(node: any) => {
				const roleId = node.attrs.role;
				const role = () => api.roles.cache.get(roleId);
				return <span class="mention-role">@{role()?.name ?? roleId}</span>;
			},
			(n) => ({ id: n.attrs.role }),
		),
		emoji: nv(
			(node: any) => {
				const url = getEmojiUrl(node.attrs.id);
				return (
					<img
						class="emoji"
						src={url}
						alt={`:${node.attrs.name}:`}
						title={`:${node.attrs.name}:`}
					/>
				);
			},
			(n) => ({ id: n.attrs.id }),
		),
	});
};
