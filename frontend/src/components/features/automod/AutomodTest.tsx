import { Dropdown } from "@/atoms/Dropdown";

export const AutomodTest = () => {
	// TODO: implement and use

	return (
		<div class="automod-test">
			<header>
				<textarea class="textarea"></textarea>
				<Dropdown
					options={[
						// TODO: add descriptions
						{
							item: "Content",
							label: "content",
							// user submitted content: messages, thread titles, voice statuses, etc
						},
						{
							item: "Member",
							label: "member",
							// text on member profiles: user names, bios, and nicknames
						},
					]}
				/>
			</header>
			<div class="results">
				<div>rules</div>
				<div>matched text (only when non media)</div>
				<div>executed actions</div>
			</div>
		</div>
	);
};
