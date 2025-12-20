import { Channel } from "sdk";
import { createSignal, Match, Switch } from "solid-js";

export const Calendar = (props: { channel: Channel }) => {
	// TODO: load calendar events from api
	// TODO: update calendar events from sync
	// TODO: schedule view (only list events)
	// TODO: render events from multiple calendars (all calendars in room)
	// TODO: render events from user list
	// TODO: render recurring events correctly
	// TODO: button to go to next/prev months
	// TODO: button to go to today
	// TODO: button to create calendar event
	// TODO: context menu for calendar events (edit, delete, copy id, log to console)
	// TODO: modal for creating/editing events
	// TODO: click event to view it
	// TODO: click day square to create new event
	// TODO: rsvp/unrsvp to events
	// TODO: list rsvps for events

	const events = new Map([
		[12, ["foo", "bar"]],
		[16, ["baz"]],
	]);

	const [view, setView] = createSignal<"week" | "month" | "timeline">("month");

	return (
		<div class="calendar">
			<header>
				<b>December 2025</b>
				<div style="flex:1"></div>
				<menu>
					<div class="filters">
						<button
							onClick={() => setView("week")}
							classList={{ active: view() === "week" }}
						>
							week
						</button>
						<button
							onClick={() => setView("month")}
							classList={{ active: view() === "month" }}
						>
							month
						</button>
						<button
							onClick={() => setView("timeline")}
							classList={{ active: view() === "timeline" }}
						>
							timeline
						</button>
					</div>
					<div class="filters" style="margin-left:4px">
						<button>prev</button>
						<button>next</button>
						<button class="primary">today</button>
					</div>
				</menu>
			</header>
			<Switch>
				<Match when={view() === "week"}>
					<CalendarWeek channel={props.channel} events={events} />
				</Match>
				<Match when={view() === "month"}>
					<CalendarMonth channel={props.channel} events={events} />
				</Match>
				<Match when={view() === "timeline"}>
					<CalendarTimeline channel={props.channel} events={events} />
				</Match>
			</Switch>
		</div>
	);
};

// TODO: strongly type events
const CalendarMonth = (props: { channel: Channel; events: any }) => {
	const offset = -1;
	const today = 9;

	return (
		<div class="month-view">
			{["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].map((i) => (
				<div class="dayofweek">{i}</div>
			))}
			{new Array(28).fill(0).map((_, i) => {
				const day0 = i + offset;
				const day = day0 > 0 ? day0 : day0 + 31;
				return (
					<div
						class="day"
						classList={{
							othermonth: day0 <= 0,
							today: day0 === today,
						}}
					>
						<span class="daynumber">{day}</span>
						{(props.events.get(day) ?? []).map((event) => (
							<span class="event">{event}</span>
						))}
					</div>
				);
			})}
		</div>
	);
};

const CalendarWeek = (props: { channel: Channel; events: any }) => {
	return "todo";
};

const CalendarTimeline = (props: { channel: Channel; events: any }) => {
	return "todo";
};
