import { createSelector, For, Show } from "solid-js";
import { Icon } from "@/atoms/Icon";
import { getCheckIcon } from "@/atoms/icons";
import { icCheck } from "@/utils/icons";

export type DocumentSort =
	| "new"
	| "activity"
	| "reactions:+1"
	| "random"
	| string;

export type DocumentView = "list" | "gallery" | string;

export type DocumentSortingProps = {
	sorting: DocumentSort;
	view: DocumentView;
	onSort(sorting: DocumentSort): void;
	onView(view: DocumentView): void;
	showRemoved: boolean;
	onToggleRemoved(show: boolean): void;
	canManage: boolean;
};

type Option = {
	id: DocumentSort;
	label: string;
};

const options: Array<Option> = [
	{ id: "new", label: "Newest documents first" },
	{ id: "activity", label: "Recently active documents" },
	{ id: "reactions:+1", label: "Reaction count" },
	{ id: "random", label: "Random ordering" },
	// NOTE: do i want hot/hot2 sorting for documents/wikis?
];

const views: Array<Option> = [
	{ id: "list", label: "List" },
	{ id: "gallery", label: "Gallery" },
];

export const DocumentSorting = (props: DocumentSortingProps) => {
	const isSortSelected = createSelector(() => props.sorting);
	const isViewSelected = createSelector(() => props.view);

	return (
		<menu class="document-sorting">
			<div class="column">
				<h3 class="dim header">sort by</h3>
				<For each={options}>
					{(option) => (
						<button
							type="button"
							class="button menu-item"
							classList={{ selected: isSortSelected(option.id) }}
							onClick={() => {
								props.onSort(option.id);
							}}
						>
							<Icon src={getCheckIcon(option.id)} />
							{option.label}
						</button>
					)}
				</For>
			</div>
			<div class="column">
				<h3 class="dim header">view as</h3>
				<For each={views}>
					{(view) => (
						<button
							type="button"
							class="button menu-item"
							classList={{ selected: isViewSelected(view.id) }}
							onClick={() => {
								props.onView(view.id);
							}}
						>
							<Icon src={getCheckIcon(view.id)} />
							{view.label}
						</button>
					)}
				</For>
				<Show when={props.canManage}>
					<br />
					<h3 class="dim header">other</h3>
					<button
						type="button"
						class="button menu-item"
						classList={{ selected: props.showRemoved }}
						onClick={() => {
							props.onToggleRemoved(!props.showRemoved);
						}}
					>
						<Icon src={icCheck} />
						Show removed documents
					</button>
				</Show>
			</div>
		</menu>
	);
};
