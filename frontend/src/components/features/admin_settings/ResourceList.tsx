import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { type Accessor, createSignal, For, type JSX, Show } from "solid-js";
import { Search } from "@/atoms/Search";

/**
 * shared layout for admin "resource" lists (users, rooms, media, ...)
 *
 * renders a search bar, filters, a table-ish header and a paginated list.
 * infinite scrolling is handled here; the caller only has to provide the
 * items and an `onLoadMore` callback.
 */
export function ResourceList<T>(props: {
	title: string;
	class?: string;
	query: Accessor<string>;
	onSearch: (query: string) => void;
	placeholder?: string;
	autofocus?: boolean;
	filters?: JSX.Element;
	header: JSX.Element;
	items: Accessor<T[]>;
	itemKey: (item: T) => string;
	renderItem: (item: T) => JSX.Element;
	hasMore?: Accessor<boolean>;
	onLoadMore?: () => void;
	loading?: Accessor<boolean>;
	empty?: string;
}) {
	const [bottom, setBottom] = createSignal<Element | undefined>();

	createIntersectionObserver(
		() => (bottom() ? [bottom()!] : []),
		(entries) => {
			if (!props.hasMore?.()) return;
			for (const entry of entries) {
				if (entry.isIntersecting) props.onLoadMore?.();
			}
		},
	);

	const emptyText = () =>
		props.loading?.() ? "loading..." : (props.empty ?? "nothing here");

	return (
		<div class={props.class ?? "admin-resource"}>
			<h2>{props.title}</h2>
			<div class="admin-filters">
				<Search
					placeholder={props.placeholder}
					value={props.query}
					onInput={props.onSearch}
					ref={(el) => {
						if (props.autofocus) queueMicrotask(() => el.focus());
					}}
				/>
				{props.filters}
			</div>
			<header>{props.header}</header>
			<Show
				when={props.items().length > 0}
				fallback={<div class="dim admin-empty">{emptyText()}</div>}
			>
				<ul>
					<For each={props.items()}>
						{(item) => (
							<li data-id={props.itemKey(item)}>{props.renderItem(item)}</li>
						)}
					</For>
				</ul>
				<Show when={props.hasMore?.()}>
					<div class="admin-list-sentinel" ref={setBottom} />
				</Show>
			</Show>
		</div>
	);
}
