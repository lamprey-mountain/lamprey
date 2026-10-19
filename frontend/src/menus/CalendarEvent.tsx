import type { CalendarEvent } from "sdk";
import { Show } from "solid-js";
import { useApi } from "@/api";
import { useCalendarPopup } from "@/components/features/calendar/Editor.tsx";
import { useModals } from "@/contexts/modal";
import { Item, Menu, Separator, Submenu } from "./Parts.tsx";

type CalendarEventMenuProps = {
	channel_id: string;
	event: CalendarEvent;
	anchor?: HTMLElement;
};

export function CalendarEventMenu(props: CalendarEventMenuProps) {
	const api = useApi();
	const { setPopup } = useCalendarPopup();
	const [, modalCtl] = useModals();

	const repeats = () => !!props.event.recurrence;

	// TODO: only edit/cancel a single instance or all future instances once
	// calendar overwrites are supported by the client
	const editEvent = () => {
		const anchor = props.anchor;
		if (!anchor) return;
		queueMicrotask(() => setPopup(anchor, "bottom-start", props.event));
	};

	// TODO: have some distinction between cancelling and deleting an event?
	const cancelEvent = () => {
		const event = props.event;
		queueMicrotask(() => {
			modalCtl.confirm("really cancel this event?", (conf) => {
				if (!conf) return;
				api.calendar.deleteEvent(props.channel_id, event.id);
			});
		});
	};

	const copyEventId = () => navigator.clipboard.writeText(props.event.id);

	const logToConsole = () =>
		console.log(JSON.parse(JSON.stringify(props.event)));

	return (
		<Menu>
			<Item>start event</Item>
			<Show
				when={repeats()}
				fallback={
					<>
						<Item onClick={editEvent}>edit event</Item>
						<Item color="danger" onClick={cancelEvent}>
							cancel event
						</Item>
					</>
				}
			>
				<Submenu content="edit event">
					<Item onClick={editEvent}>this instance</Item>
					<Item onClick={editEvent}>all future instances</Item>
					<Item onClick={editEvent}>all events</Item>
				</Submenu>
				<Submenu content="cancel event" color="danger">
					<Item color="danger">this instance</Item>
					<Item color="danger">all future instances</Item>
					<Item color="danger" onClick={cancelEvent}>
						all events
					</Item>
				</Submenu>
			</Show>
			<Submenu content="copy link">
				<Item>this instance</Item>
				<Item>all events in this series</Item>
			</Submenu>
			<Separator />
			<Item onClick={copyEventId}>copy event id</Item>
			<Item>copy instance seq</Item>
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}
