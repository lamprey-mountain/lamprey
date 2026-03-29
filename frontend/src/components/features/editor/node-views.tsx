import { getOwner, runWithOwner, type VoidComponent } from "solid-js";
import { render } from "solid-js/web";
import { getTwemoji, getTwemojiUrl } from "../../../emoji.ts";
import { getEmojiUrl } from "../../../media/util.tsx";

export const createNodeViews = () => {
	const owner = getOwner();

	return <T extends Record<string, any>>(
		propsFn: (node: any) => T,
		Component: VoidComponent<T>,
	) =>
		(node: any) => {
			const dom = document.createElement("span");
			dom.classList.add("node-view-wrapper");

			// Capture props synchronously before rendering to avoid
			// reactive tracking inside the render callback
			const props = propsFn(node);
			let currentProps = props;

			const dispose = render(
				() => runWithOwner(owner, () => <Component {...currentProps} />),
				dom,
			);

			return {
				dom,
				update: (newNode: any) => {
					// Update props when the node changes
					currentProps = propsFn(newNode);
					// Note: We don't re-render here to avoid creating new computations.
					// The component will re-render when its parent re-renders.
				},
				destroy: () => dispose(),
			};
		};
};

export const createEditorNodeViews = () => {
	const nv = createNodeViews();

	return () => ({
		mention: nv(
			(n) => ({
				id: n.attrs.user,
				name: n.attrs.name ?? n.attrs.user ?? "...",
			}),
			(props) => {
				return <span class="mention mention-user">@{props.name}</span>;
			},
		),
		mentionChannel: nv(
			(n) => ({
				id: n.attrs.channel,
				name: n.attrs.name ?? n.attrs.channel ?? "...",
			}),
			(props) => {
				return <span class="mention mention-channel">#{props.name}</span>;
			},
		),
		mentionRole: nv(
			(n) => ({
				id: n.attrs.role,
				name: n.attrs.name ?? n.attrs.role ?? "...",
			}),
			(props) => {
				return <span class="mention mention-role">@{props.name}</span>;
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
