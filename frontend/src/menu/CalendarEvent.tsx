import { Item, Menu, Separator, Submenu } from "./Parts.tsx";

type CalendarEventMenuProps = {};

export function CalendarEventMenu(_props: CalendarEventMenuProps) {
	// TODO: make this work
	// TODO: only show Item instead of Submenu if event does not repeat

	return (
		<Menu>
			<Item>start event</Item>
			<Submenu content="edit event">
				<Item>this event</Item>
				<Item>all future events</Item>
				<Item>all events</Item>
			</Submenu>
			<Submenu content="cancel event" color="danger">
				<Item color="danger">this event</Item>
				<Item color="danger">all future events</Item>
				<Item color="danger">all events</Item>
			</Submenu>
			<Submenu content="copy link">
				<Item>this instance</Item>
				<Item>all events in this series</Item>
			</Submenu>
			<Separator />
			<Item>copy calendar event id</Item>
			<Item>copy calendar event seq</Item>
			<Item>log to console</Item>
		</Menu>
	);
}
