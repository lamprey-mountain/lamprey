import { ParentProps } from "solid-js";

export type SearchAutocompleteProps = {
	// items: AutocompleteItem[];
	// onSelect: (...) => void;
};

export const SearchAutocomplete = (props: SearchAutocompleteProps) => {
	return (
		<div
			class="search-autocomplete"
			onClick={(e) => {
				e.stopPropagation();
				e.stopImmediatePropagation();
			}}
			// onPointerDown={props.onPointerDown}
			// onBlur={props.onBlur}
		>
			todo
		</div>
	);
};

// const FilterListItem = (props: ParentProps<{
//   onSelect: () => void;
// }>) => {
//   const handleSelect = (e: Event) => {
//     e.preventDefault();
//     props.onSelect();
//   };

//   return (
//     <li
//       class="filter-item"
//       onMouseDown={handleSelect}
//       onKeyDown={(e) => {
//         if (e.key === "Enter" || e.key === " ") handleSelect(e);
//       }}
//     >
//       {props.children}
//     </li>
//   );
// };
