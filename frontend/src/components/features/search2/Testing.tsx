import { createSearch } from "./context";
import { SearchInput } from "./SearchInput";

export const Search2Testing = () => {
	const config = createSearch({
		filters: [{ name: "author" }],
	});

	return (
		<div>
			<SearchInput placeholder="asdf" config={config} />
		</div>
	);
};
