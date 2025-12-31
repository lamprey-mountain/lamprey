import { Channel } from "sdk";
import { createMemo, createSignal, Match, Switch } from "solid-js";

const CalendarEvent = (props: {}) => {
	// TODO: show icons next to each input
	// TODO: show tooltip for each input
	return (
		<div>
			<menu>
				<button>event</button>
				{/* TODO: only show if this event repeats */}
				{/* TODO: show n if there is a limited number of instances */}
				<button>(n) instances</button>
				{/* TODO: show number of participants */}
				<button>123 participants</button>
			</menu>
			<div style="display:flex;flex-direction:column">
				<h3>event</h3>
				<input placeholder="event name" />
				<input type="date" />
				<div style="display:flex">
					{/* starts..ends */}
					<input type="time" />
					<input type="time" />
				</div>
				<hr />
				<div>all day</div>
				{/* TODO: checkbox for all day option */}
				<div>timezone</div>
				{/* TODO: remove timezone? this could be a user config thing instead? */}
				<div>recurrence</div>
				{/* TODO: on click, open context menu with options: every day, every week, every other week, every year, every weekday, annually, custom... */}
				<hr />
				<input type="text" placeholder="location" />
				<input type="url" placeholder="url" />
				<textarea placeholder="description"></textarea>
				<hr />
				<div>reminders</div>
				{/* TODO: list reminders */}
				{/* TODO: show x button next to each reminder to close it */}
				{/* TODO: on click, open context menu with options: at start of event, 15 minutes before, 1 hour before, 1 day before, 3 days before, 1 week before, custom... */}
				{/* TODO: show savebar when dirty */}
			</div>
			<div>
				<h3>instances</h3>
				{/* TODO: show a list of event instances */}
			</div>
			<div>
				<h3>participants</h3>
				{/* TODO: show a list of participants */}
			</div>
		</div>
	);
};

/*
calendar event context menu:
start event
edit event (only this event, all events in series)
cancel event (only this event, all events in series)
copy link (only this event, all events in series)
---
copy event id
copy event seq
log to console
*/

/*
custom repeat modal:
- repeat every {n} {day|week|month|year}
- for week: on {su,mo,tu,we,th,fr,sa}
- for month: {on the {day}th, on the {n}th {weekday}}
- ends {never, on {time}, after {n} times}
*/

export const Calendar = (props: { channel: Channel }) => {
	// TODO: load calendar events from api
	// TODO: update calendar events from sync
	// TODO: schedule view (only list events)
	// TODO: render events from multiple calendars (all calendars in room)
	// TODO: render events from user list
	// TODO: render recurring events correctly
	// TODO: button to create calendar event
	// TODO: context menu for calendar events (edit, delete, copy id, log to console)
	// TODO: modal for creating/editing events
	// TODO: click event to view it
	// TODO: click day square to create new event
	// TODO: rsvp/unrsvp to events
	// TODO: list rsvps for events

	const [currentDate, setCurrentDate] = createSignal(new Date(2025, 11, 1));

	const month = () =>
		currentDate().toLocaleString("default", { month: "long" });
	const year = () => currentDate().getFullYear();

	const prevMonth = () => {
		setCurrentDate(
			new Date(currentDate().setMonth(currentDate().getMonth() - 1)),
		);
	};

	const nextMonth = () => {
		setCurrentDate(
			new Date(currentDate().setMonth(currentDate().getMonth() + 1)),
		);
	};

	const goToToday = () => {
		setCurrentDate(new Date());
	};

	const events = new Map([
		[12, ["foo", "bar"]],
		[16, ["baz"]],
	]);

	const [view, setView] = createSignal<"week" | "month" | "timeline">("month");

	return (
		<div class="calendar">
			<header>
				<b>
					{month()} {year()}
				</b>
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
						<button onClick={prevMonth}>prev</button>
						<button onClick={nextMonth}>next</button>
						<button class="primary" onClick={goToToday}>
							today
						</button>
					</div>
				</menu>
			</header>
			<Switch>
				<Match when={view() === "week"}>
					<CalendarWeek channel={props.channel} events={events} />
				</Match>
				<Match when={view() === "month"}>
					<CalendarMonth
						channel={props.channel}
						events={events}
						date={currentDate()}
					/>
				</Match>
				<Match when={view() === "timeline"}>
					<CalendarTimeline channel={props.channel} events={events} />
				</Match>
			</Switch>
			<CalendarEvent />
		</div>
	);
};

// TODO: strongly type events
const CalendarMonth = (props: {
	channel: Channel;
	events: any;
	date: Date;
}) => {
	const dayStartsAt = () => 0;
	const year = () => props.date.getFullYear();
	const month = () => props.date.getMonth(); // 0-indexed

	const calendarDays = () => {
		const days = [];
		const firstDay = new Date(year(), month(), 1);

		const daysInMonth = new Date(year(), month() + 1, 0).getDate();
		const firstDayWeekday = (firstDay.getDay() - dayStartsAt() + 7) % 7;

		const daysInPrevMonth = new Date(year(), month(), 0).getDate();

		// Days from previous month
		for (let i = firstDayWeekday; i > 0; i--) {
			days.push({ day: daysInPrevMonth - i + 1, isOtherMonth: true });
		}

		// Days from current month
		for (let i = 1; i <= daysInMonth; i++) {
			days.push({ day: i, isOtherMonth: false });
		}

		// Days from next month
		const totalDays = days.length;
		const remaining = 42 - totalDays; // Potential 6 weeks
		for (let i = 1; i <= remaining; i++) {
			days.push({ day: i, isOtherMonth: true });
		}

		// Hide last week if all days are from next month
		const lastWeekStartIndex = days.length - 7;
		const lastWeek = days.slice(lastWeekStartIndex);
		if (lastWeek.every((day) => day.isOtherMonth)) {
			return days.slice(0, lastWeekStartIndex);
		}

		return days;
	};

	const today = new Date();
	const isToday = (day: number) => {
		return (
			day === today.getDate() &&
			month() === today.getMonth() &&
			year() === today.getFullYear()
		);
	};

	const displayDaysOfWeek = createMemo(() => {
		const daysOfWeek = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
		const start = dayStartsAt();
		return [...daysOfWeek.slice(start), ...daysOfWeek.slice(0, start)];
	});

	return (
		<div class="month-view">
			{displayDaysOfWeek().map((i) => <div class="dayofweek">{i}</div>)}
			{calendarDays().map((d) => {
				return (
					<div
						class="day"
						classList={{
							othermonth: d.isOtherMonth,
							today: !d.isOtherMonth && isToday(d.day),
						}}
					>
						<span class="daynumber">{d.day}</span>
						{!d.isOtherMonth &&
							(props.events.get(d.day) ?? []).map((event: string) => (
								<span class="event">{event}</span>
							))}
					</div>
				);
			})}
		</div>
	);
};

const CalendarWeek = (props: { channel: Channel; events: any }) => {
	// TODO: implement week view
	return (
		<div class="week-view">
			<p>Week view coming soon...</p>
		</div>
	);
};

const CalendarTimeline = (props: { channel: Channel; events: any }) => {
	// TODO: implement timeline view
	return (
		<div class="timeline-view">
			<p>Timeline view coming soon...</p>
		</div>
	);
};
