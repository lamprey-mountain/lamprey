import { Channel } from "sdk";
import { createMemo, createSignal, Match, Switch } from "solid-js";

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
