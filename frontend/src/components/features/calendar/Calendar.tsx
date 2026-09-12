import type { CalendarEvent, Channel } from "sdk";
import {
	createEffect,
	createResource,
	createSignal,
	Match,
	Switch,
} from "solid-js";
import { useApi } from "@/api";
import { Dropdown } from "@/atoms/Dropdown";
import { ChatHeader } from "../chat/ChatHeader";
import { CalendarProvider, createCalendar } from "./context";
import { useCalendarPopup } from "./Editor";
import { CalendarMonth } from "./Month";
import { CalendarTimeline } from "./Timeline";
import { CalendarWeek } from "./Week";

export {
	type CalendarPopup,
	CalendarPopupProvider,
	PopupEventEditor,
	useCalendarPopup,
} from "./Editor";

export const Calendar = (props: { channel: Channel }) => {
	const api = useApi();

	const calendar = createCalendar();
	const currentDate = () => calendar.date;
	const {
		popup: calendarPopup,
		setPopup,
		closePopup,
		setChannelId,
	} = useCalendarPopup();

	// Set channel_id when component mounts or channel_id changes
	createEffect(() => {
		setChannelId(props.channel.id);
	});

	const month = () =>
		currentDate().toLocaleString("default", { month: "long" });
	const year = () => currentDate().getFullYear();

	// FIXME: update events on sync
	const [events] = createResource(
		() => props.channel.id,
		async (cid) => {
			const events = await api.calendar.listEvents(cid);
			return events;
		},
	);

	const [view, setView] = createSignal<"week" | "month" | "timeline">("month");

	// Open popup for new event when clicking a day
	const handleDayClick = (day: number, el: HTMLElement) => {
		const current = calendarPopup();
		if (current?.ref === el) {
			closePopup();
			return;
		}

		const newEvent = {
			name: "",
			start: new Date(
				currentDate().getFullYear(),
				currentDate().getMonth(),
				day,
				9,
				0,
			),
			end: new Date(
				currentDate().getFullYear(),
				currentDate().getMonth(),
				day,
				10,
				0,
			),
			allDay: false,
			timezone: "UTC",
		};
		setPopup(el, "bottom-start", newEvent);
	};

	// Open popup for editing event when clicking an event
	const handleEventClick = (
		event: CalendarEvent,
		day: number, // TODO: remove param
		el: HTMLElement,
	) => {
		const current = calendarPopup();
		if (current?.ref === el) {
			closePopup();
			return;
		}

		// TODO: use CalendarEvent instead of custom type
		const existingEvent = {
			id: event.id,
			name: event.title,
			start: new Date(event.starts_at),
			end: event.ends_at ? new Date(event.ends_at) : null,
			allDay: false, // TODO: support this
			timezone: event.timezone ?? "UTC", // TODO: use channel/calendar timezone if null
			recurrence: "", // TODO: support this
			location: event.location ?? "",
			url: event.url ?? "",
			description: event.description ?? "",
			reminders: [],
			instances: [],
			participants: [],
		};
		setPopup(el, "bottom-start", existingEvent);
	};

	return (
		<>
			<ChatHeader channel={props.channel} />
			<div class="calendar-channel">
				<header class="calendar-header">
					<b>
						{month()} {year()}
					</b>
					<div style="flex:1"></div>
					<menu>
						<Dropdown
							options={[
								{ item: "week", label: "week" },
								{ item: "month", label: "month" },
								{ item: "timeline", label: "timeline" },
								// TODO: week view with specific number of days per week
							]}
							selected="month"
							required
							onSelect={setView}
						/>
						<div class="filters" style="margin-left:4px">
							<button type="button" class="button" onClick={calendar.prevMonth}>
								prev
							</button>
							<button type="button" class="button" onClick={calendar.nextMonth}>
								next
							</button>
							<button
								type="button"
								class="button primary"
								onClick={calendar.gotoToday}
							>
								today
							</button>
						</div>
					</menu>
				</header>
				<CalendarProvider state={calendar}>
					<Switch>
						<Match when={view() === "week"}>
							<CalendarWeek />
						</Match>
						<Match when={view() === "month"}>
							<CalendarMonth
								channel={props.channel}
								events={events() ?? []}
								onDayClick={handleDayClick}
								onEventClick={handleEventClick}
							/>
						</Match>
						<Match when={view() === "timeline"}>
							<CalendarTimeline />
						</Match>
					</Switch>
				</CalendarProvider>
			</div>
		</>
	);
};
