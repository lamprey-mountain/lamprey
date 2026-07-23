import { createSelector, For } from "solid-js";
import { Icon } from "@/atoms/Icon";
import { icCheck, icReply } from "@/utils/icons.ts";

export type CommentSort = "new" | "old" | "activity";

export type CommentSortingProps = {
	sorting: CommentSort;
	onSort(sorting: CommentSort): void;
};

const options: {
	id: CommentSort;
	label: string;
	description: string;
	icon: any;
}[] = [
	{
		id: "new",
		label: "newest",
		description: "newest comments first",
		icon: icCheck,
	},
	{
		id: "old",
		label: "oldest",
		description: "oldest comments first",
		icon: icCheck,
	},
	{
		id: "activity",
		label: "recent activity",
		description: "comments that have been recently replied or reacted to",
		icon: icReply,
	},

	// TODO: more comment sorting
	// { item: "reactions:+1", label: "most +1 reactions" },
	// { item: "random", label: "random ordering" },
	// { item: "hot", label: "mystery algorithm 1" },
	// { item: "hot2", label: "mystery algorithm 2" },
	// NOTE: hacker news algorithm
	//   score = points / ((time + 2) ** gravity)
	//   time = how old the post is in hours(?)
	//   gravity = 1.8
];

export const CommentSorting = (props: CommentSortingProps) => {
	const isSelected = createSelector(() => props.sorting);

	return (
		<menu class="forum2-sorting forum2-comment-sorting">
			<div class="column">
				<h3 class="dim header">sort by</h3>
				<For each={options}>
					{(option) => (
						<button
							type="button"
							class="button menu-item"
							classList={{ selected: isSelected(option.id) }}
							onClick={() => props.onSort(option.id)}
						>
							<Icon src={option.icon} />
							<div>
								<div>{option.label}</div>
								<div class="dim">{option.description}</div>
							</div>
						</button>
					)}
				</For>
			</div>
		</menu>
	);
};
