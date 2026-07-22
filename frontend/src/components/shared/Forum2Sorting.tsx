import { Accessor, createSelector, For, Show } from "solid-js";
import type { Channel } from "ts-sdk";
import { Icon } from "@/atoms/Icon";
import { getCheckIcon } from "@/atoms/icons";
import { icCheck } from "@/utils/icons";

export type Forum2Sort =
	| "new"
	| "activity"
	| "reactions:+1"
	| "random"
	| "hot"
	| "hot2"
	| string;

export type Forum2View = "list" | "gallery" | string;

export type Forum2SortingProps = {
	sorting: Forum2Sort;
	view: Forum2View;
	onSort(sorting: Forum2Sort): void;
	onView(view: Forum2View): void;
	showRemoved: boolean;
	onToggleRemoved(show: boolean): void;
	canManage: boolean;
};

type Option = {
	id: Forum2Sort;
	label: string;
};

const options: Array<Option> = [
	{ id: "new", label: "Newest threads first" },
	{ id: "activity", label: "Recently active threads" },
	{ id: "reactions:+1", label: "Expected to be helpful" },
	{ id: "random", label: "Random ordering" },
	{ id: "hot", label: "Hot" },
	{ id: "hot2", label: "Hot 2" },
];

const views: Array<Option> = [
	{ id: "list", label: "List" },
	{ id: "gallery", label: "Gallery" },
];

export const Forum2Sorting = (props: Forum2SortingProps) => {
	const isSortSelected = createSelector(() => props.sorting);
	const isViewSelected = createSelector(() => props.view);

	return (
		<menu class="forum2-sorting">
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
						Show removed threads
					</button>
				</Show>
			</div>
		</menu>
	);
};
