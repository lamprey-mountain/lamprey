import { Channel } from "sdk";
import {
	createContext,
	createEffect,
	createMemo,
	createSignal,
	Match,
	type ParentProps,
	Show,
	Switch,
	useContext,
} from "solid-js";
import { createStore } from "solid-js/store";
import { Checkbox, XMark } from "./icons";
import { CheckboxOption } from "./atoms/CheckboxOption";
import { Dropdown, type DropdownItem } from "./Dropdown";

export type CalendarPopup = {
	ref: HTMLElement | null;
	id: "event-editor";
	props: {
		channel_id: string;
		event?: {
			id?: string;
			name: string;
			start: Date;
			end: Date;
			allDay: boolean;
			timezone: string;
			recurrence?: string;
			location?: string;
			url?: string;
			description?: string;
			reminders?: string[];
			instances?: string[];
			participants?: string[];
		};
	};
	placement: "bottom-end" | "top-end" | "bottom-start" | "top-start";
};

type CalendarPopupContextType = {
	popup: () => CalendarPopup | null;
	setPopup: (
		ref: HTMLElement | null,
		placement: CalendarPopup["placement"],
		event?: CalendarPopup["props"]["event"],
	) => void;
	closePopup: () => void;
	getPopupRef: () => HTMLElement | undefined;
	setChannelId: (channel_id: string) => void;
};

const CalendarPopupContext = createContext<CalendarPopupContextType>();

export const CalendarPopupProvider = (props: ParentProps) => {
	const [popup, setPopup] = createSignal<CalendarPopup | null>(null);
	const [channelId, setChannelIdState] = createSignal<string>("");

	const setCalendarPopup = (
		ref: HTMLElement | null,
		placement: CalendarPopup["placement"],
		event?: CalendarPopup["props"]["event"],
	) => {
		if (!ref) {
			setPopup(null);
			return;
		}
		setPopup({
			ref,
			id: "event-editor",
			props: {
				channel_id: channelId(),
				event,
			},
			placement,
		});
	};

	const closeCalendarPopup = () => {
		setPopup(null);
	};

	const setChannelId = (channel_id: string) => {
		setChannelIdState(channel_id);
		setPopup((current) => {
			if (!current) return null;
			return {
				...current,
				props: {
					...current.props,
					channel_id,
				},
			};
		});
	};

	return (
		<CalendarPopupContext.Provider
			value={{
				popup: () => popup(),
				setPopup: setCalendarPopup,
				closePopup: closeCalendarPopup,
				getPopupRef: () => popup()?.ref ?? undefined,
				setChannelId,
			}}
		>
			{props.children}
		</CalendarPopupContext.Provider>
	);
};

export const useCalendarPopup = (): CalendarPopupContextType => {
	const context = useContext(CalendarPopupContext);
	if (!context) {
		throw new Error(
			"useCalendarPopup must be used within a CalendarPopupProvider",
		);
	}
	return context;
};

export const PopupEventEditor = (props: {
	channel_id: string;
	event?: {
		id?: string;
		name: string;
		start: Date;
		end: Date;
		allDay: boolean;
		timezone: string;
		recurrence?: string;
		location?: string;
		url?: string;
		description?: string;
		reminders?: string[];
		instances?: string[];
		participants?: string[];
	};
	onClose: () => void;
}) => {
	const [activeTab, setActiveTab] = createSignal<
		"event" | "instances" | "participants"
	>("event");
	const [formData, setFormData] = createStore({
		name: props.event?.name || "",
		start: props.event?.start ? new Date(props.event.start) : new Date(),
		end: props.event?.end ? new Date(props.event.end) : new Date(),
		allDay: props.event?.allDay || false,
		timezone: props.event?.timezone || "UTC",
		recurrence: props.event?.recurrence || "",
		location: props.event?.location || "",
		url: props.event?.url || "",
		description: props.event?.description || "",
		reminders: props.event?.reminders || [],
	});

	// Handle external updates (e.g. clicking another day in the calendar)
	let lastId = props.event?.id;
	createEffect(() => {
		const event = props.event;
		if (!event) return;

		const isSameEvent = event.id === lastId;
		lastId = event.id;

		setFormData({
			start: new Date(event.start),
			end: new Date(event.end),
			// Only reset other fields if it's a completely different event (different ID)
			...(!isSameEvent
				? {
					name: event.name || "",
					allDay: event.allDay || false,
					timezone: event.timezone || "UTC",
					recurrence: event.recurrence || "",
					location: event.location || "",
					url: event.url || "",
					description: event.description || "",
					reminders: event.reminders || [],
				}
				: {}),
		});
	});

	const handleChange = (field: string, value: any) => {
		setFormData(field as any, value);
	};

	return (
		<div class="calendar-event-popup">
			<div class="popup-header">
				<h2>
					{formData.name || (props.event?.id ? "Edit Event" : "New Event")}
				</h2>
				<button class="popup-close" onClick={props.onClose}>
					<XMark seed={props.event?.id || "new"} />
				</button>
			</div>

			<Show when={props.event?.id}>
				<div class="popup-tabs">
					<button
						class={`popup-tab ${activeTab() === "event" ? "active" : ""}`}
						onClick={() => setActiveTab("event")}
					>
						Event
					</button>
					<Show when={props.event?.recurrence}>
						<button
							class={`popup-tab ${activeTab() === "instances" ? "active" : ""}`}
							onClick={() => setActiveTab("instances")}
						>
							{props.event?.instances && props.event.instances.length > 0
								? `${props.event.instances.length} instances`
								: "Instances"}
						</button>
					</Show>
					<button
						class={`popup-tab ${
							activeTab() === "participants" ? "active" : ""
						}`}
						onClick={() => setActiveTab("participants")}
					>
						{props.event?.participants && props.event.participants.length > 0
							? `${props.event.participants.length} participants`
							: "Participants"}
					</button>
				</div>
			</Show>

			<div class="popup-content">
				{/* Event Tab */}
				<div class={`tab-content ${activeTab() === "event" ? "active" : ""}`}>
					<div class="popup-form">
						<div class="popup-form-group">
							<label>Event Name</label>
							<input
								type="text"
								placeholder="Event name"
								value={formData.name}
								onInput={(e) => handleChange("name", e.currentTarget.value)}
							/>
						</div>

						<div class="popup-form-row">
							<div class="popup-form-group">
								<label>Date</label>
								<input
									type="date"
									value={formData.start.toISOString().split("T")[0]}
									onInput={(e) => {
										const date = new Date(e.currentTarget.value);
										handleChange("start", date);
									}}
								/>
							</div>
							<div class="popup-form-group">
								<label>Timezone</label>
								<Dropdown
									selected={formData.timezone}
									onSelect={(v) => handleChange("timezone", v)}
									options={[
										{ item: "UTC", label: "UTC" },
										{ item: "America/New_York", label: "Eastern" },
										{ item: "America/Chicago", label: "Central" },
										{ item: "America/Denver", label: "Mountain" },
										{ item: "America/Los_Angeles", label: "Pacific" },
										{ item: "Europe/London", label: "London" },
										{ item: "Europe/Paris", label: "Paris" },
										{ item: "Asia/Tokyo", label: "Tokyo" },
									]}
								/>
							</div>
						</div>

						<div class="popup-form-row">
							<div class="popup-form-group">
								<label>Start</label>
								<input
									type="time"
									value={formData.start.toTimeString().slice(0, 5)}
									onInput={(e) => {
										const [hours, minutes] = e.currentTarget.value.split(":")
											.map(Number);
										const newDate = new Date(formData.start);
										newDate.setHours(hours, minutes);
										handleChange("start", newDate);
									}}
								/>
							</div>
							<div class="popup-form-group">
								<label>End</label>
								<input
									type="time"
									value={formData.end.toTimeString().slice(0, 5)}
									onInput={(e) => {
										const [hours, minutes] = e.currentTarget.value.split(":")
											.map(Number);
										const newDate = new Date(formData.end);
										newDate.setHours(hours, minutes);
										handleChange("end", newDate);
									}}
								/>
							</div>
						</div>

						<CheckboxOption
							id="allDay"
							checked={formData.allDay}
							onChange={(checked) => handleChange("allDay", checked)}
							seed="allDay"
						>
							<Checkbox checked={formData.allDay} seed="allDay" />
							<label for="allDay">All day</label>
						</CheckboxOption>

						<div class="popup-form-group">
							<label>Recurrence</label>
							<Dropdown
								selected={formData.recurrence}
								onSelect={(v) => handleChange("recurrence", v)}
								options={[
									{ item: "", label: "None" },
									{ item: "daily", label: "Every day" },
									{ item: "weekly", label: "Every week" },
									{ item: "biweekly", label: "Every other week" },
									{ item: "monthly", label: "Every month" },
									{ item: "yearly", label: "Every year" },
									{ item: "weekdays", label: "Every weekday" },
								]}
							/>
						</div>

						<div class="popup-form-group">
							<label>Location</label>
							<input
								type="text"
								placeholder="Location"
								value={formData.location}
								onInput={(e) => handleChange("location", e.currentTarget.value)}
							/>
						</div>

						<div class="popup-form-group">
							<label>URL</label>
							<input
								type="url"
								placeholder="https://..."
								value={formData.url}
								onInput={(e) => handleChange("url", e.currentTarget.value)}
							/>
						</div>

						<div class="popup-form-group">
							<label>Description</label>
							<textarea
								placeholder="Description"
								value={formData.description}
								onInput={(e) =>
									handleChange("description", e.currentTarget.value)}
							/>
						</div>

						<div class="popup-form-group">
							<label>Reminder</label>
							<Dropdown
								selected={formData.reminders[0] || ""}
								onSelect={(v) => handleChange("reminders", v ? [v] : [])}
								options={[
									{ item: "", label: "None" },
									{ item: "at_start", label: "At start" },
									{ item: "15min", label: "15 min before" },
									{ item: "1hour", label: "1 hour before" },
									{ item: "1day", label: "1 day before" },
									{ item: "3days", label: "3 days before" },
									{ item: "1week", label: "1 week before" },
								]}
							/>
						</div>
					</div>
				</div>

				{/* Instances Tab */}
				<div
					class={`tab-content ${activeTab() === "instances" ? "active" : ""}`}
				>
					{props.event?.instances
						? (
							<div class="popup-form">
								<h3>Event Instances</h3>
								<p>
									This event has {props.event.instances.length}{" "}
									instances. Click on individual instances to edit them.
								</p>
							</div>
						)
						: (
							<p>
								No instances configured. Set a recurrence pattern to create
								recurring instances.
							</p>
						)}
				</div>

				{/* Participants Tab */}
				<div
					class={`tab-content ${
						activeTab() === "participants" ? "active" : ""
					}`}
				>
					{props.event?.participants
						? (
							<div class="popup-form">
								<h3>Event Participants</h3>
								<p>
									This event has {props.event.participants.length} participants.
								</p>
							</div>
						)
						: <p>No participants added yet.</p>}
				</div>
			</div>

			<div class="popup-footer">
				<button class="popup-cancel-btn" onClick={props.onClose}>
					Cancel
				</button>
				<button class="popup-save-btn" onClick={props.onClose}>
					Save
				</button>
			</div>
		</div>
	);
};

export const Calendar = (props: { channel: Channel }) => {
	const [currentDate, setCurrentDate] = createSignal(new Date(2025, 11, 1));
	const { popup: calendarPopup, setPopup, closePopup, setChannelId } =
		useCalendarPopup();

	// Set channel_id when component mounts or channel_id changes
	createEffect(() => {
		setChannelId(props.channel.id);
	});

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
		eventName: string,
		day: number,
		el: HTMLElement,
	) => {
		const current = calendarPopup();
		if (current?.ref === el) {
			closePopup();
			return;
		}

		const existingEvent = {
			id: `event-${day}-${eventName}`,
			name: eventName,
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
			recurrence: "",
			location: "",
			url: "",
			description: "",
			reminders: [],
			instances: [],
			participants: [],
		};
		setPopup(el, "bottom-start", existingEvent);
	};

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
						onDayClick={handleDayClick}
						onEventClick={handleEventClick}
					/>
				</Match>
				<Match when={view() === "timeline"}>
					<CalendarTimeline channel={props.channel} events={events} />
				</Match>
			</Switch>
		</div>
	);
};

const CalendarMonth = (props: {
	channel: Channel;
	events: any;
	date: Date;
	onDayClick: (day: number, el: HTMLElement) => void;
	onEventClick: (eventName: string, day: number, el: HTMLElement) => void;
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
		return day === today.getDate() && month() === today.getMonth() &&
			year() === today.getFullYear();
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
						ref={(el) => {
							if (el && !d.isOtherMonth) {
								el.addEventListener("click", () => props.onDayClick(d.day, el));
							}
						}}
					>
						<span class="daynumber">{d.day}</span>
						{!d.isOtherMonth &&
							(props.events.get(d.day) ?? []).map((event: string) => (
								<span
									class="event"
									ref={(el) => {
										if (el) {
											el.addEventListener("click", (e) => {
												e.stopPropagation();
												props.onEventClick(event, d.day, el);
											});
										}
									}}
								>
									{event}
								</span>
							))}
					</div>
				);
			})}
		</div>
	);
};

const CalendarWeek = (props: { channel: Channel; events: any }) => {
	return (
		<div class="week-view">
			<p>Week view coming soon...</p>
		</div>
	);
};

const CalendarTimeline = (props: { channel: Channel; events: any }) => {
	return (
		<div class="timeline-view">
			<p>Timeline view coming soon...</p>
		</div>
	);
};
