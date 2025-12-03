export const Calendar = () => {
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

	const offset = -1;
	const today = 9;
	const events = new Map([
		[12, ["foo", "bar"]],
		[16, ["baz"]],
	]);
	return (
		<div class="calendar">
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
						{(events.get(day) ?? []).map((event) => (
							<span class="event">{event}</span>
						))}
					</div>
				);
			})}
		</div>
	);
};
