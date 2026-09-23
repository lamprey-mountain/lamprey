import type { CalendarEvent } from "sdk";
import type {
	CalendarEventCreate,
	CalendarEventPatch,
	Recurrence,
	RecurrenceFrequency,
} from "sdk/types";
import {
	createContext,
	createEffect,
	createSignal,
	For,
	Match,
	type ParentProps,
	Show,
	Switch,
	useContext,
} from "solid-js";
import { createStore } from "solid-js/store";
import { useApi } from "@/api";
import { CheckboxOption } from "@/atoms/CheckboxOption";
import { Dropdown } from "@/atoms/Dropdown";
import { Checkbox, XMark } from "@/atoms/icons";
import { getDate } from "@/utils/general";
import { WEEKDAYS } from "./utils";

export type CalendarPopup = {
	ref: HTMLElement | null;
	id: "event-editor";
	props: {
		channel_id: string;
		event?: CalendarEvent;
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
	event?: CalendarEvent;
	onClose: () => void;
}) => {
	const [activeTab, setActiveTab] = createSignal<
		"event" | "instances" | "participants"
	>("event");
	const [formData, setFormData] = createStore({
		name: props.event?.title || "",
		start: props.event?.starts_at ? getDate(props.event.starts_at) : new Date(),
		end: props.event?.ends_at ? getDate(props.event.ends_at) : null,
		allDay: false, // TODO: support in CalendarEvent
		timezone: props.event?.timezone || "UTC",
		recurrence: props.event?.recurrence ?? null,
		location: props.event?.location || "",
		url: props.event?.url || "",
		description: props.event?.description || "",
		reminders: [] as string[], // TODO: save in user preferences, handle reminders in backend notification service
	});

	// Handle external updates (e.g. clicking another day in the calendar)
	let lastId = props.event?.id;
	createEffect(() => {
		const event = props.event;
		if (!event) return;

		const isSameEvent = event.id === lastId;
		lastId = event.id;

		setFormData({
			start: getDate(event.starts_at),
			end: event.ends_at ? getDate(event.ends_at) : null,
			// Only reset other fields if it's a completely different event (different ID)
			...(!isSameEvent
				? {
						name: event.title || "",
						allDay: false, // TODO: support in CalendarEvent
						timezone: event.timezone || "UTC",
						recurrence: event.recurrence ?? null,
						location: event.location || "",
						url: event.url || "",
						description: event.description || "",
						reminders: [] as string[], // TODO: save in user preferences, handle reminders in backend notification service
					}
				: {}),
		});
	});

	const handleChange = <K extends keyof typeof formData>(
		field: K,
		value: (typeof formData)[K],
	) => {
		setFormData(field, value);
	};

	const api = useApi();
	const save = async () => {
		const existingId = props.event?.id;
		if (existingId) {
			api.client.http.PATCH("/api/v1/calendar/{channel_id}/event/{event_id}", {
				params: {
					path: { channel_id: props.channel_id, event_id: existingId },
				},
				body: {
					title: formData.name, // TODO: require
					description: formData.description || null,
					recurrence: formData.recurrence,
					location: formData.location || null,
					url: formData.url || null,
					starts_at: formData.start.toISOString(),
					ends_at: formData.end?.toISOString(), // TODO: make optional
				} as CalendarEventPatch,
			});
		} else {
			api.client.http.POST("/api/v1/calendar/{channel_id}/event", {
				params: { path: { channel_id: props.channel_id } },
				body: {
					title: formData.name, // TODO: require
					description: formData.description || null,
					location: formData.location || null,
					recurrence: formData.recurrence,
					timezone: null, // TODO: better timezone input
					url: formData.url || null,
					starts_at: formData.start.toISOString(),
					ends_at: formData.end?.toISOString(), // TODO: make optional
				} as CalendarEventCreate,
			});
		}
	};

	// TODO: fetch this from api
	const instances = () => [];
	const participants = () => [];

	return (
		<div class="calendar-event-popup">
			<div class="popup-header">
				<h2>
					{formData.name || (props.event?.id ? "Edit Event" : "New Event")}
				</h2>
				<button type="button" class="popup-close" onClick={props.onClose}>
					<XMark seed={props.event?.id || "new"} />
				</button>
			</div>

			<Show when={props.event?.id}>
				<div class="popup-tabs">
					<button
						type="button"
						class={`popup-tab ${activeTab() === "event" ? "active" : ""}`}
						onClick={() => setActiveTab("event")}
					>
						Event
					</button>
					<Show when={props.event?.recurrence}>
						<button
							type="button"
							class={`popup-tab ${activeTab() === "instances" ? "active" : ""}`}
							onClick={() => setActiveTab("instances")}
						>
							{instances().length > 0
								? `${instances().length} instances`
								: "Instances"}
						</button>
					</Show>
					<button
						type="button"
						class={`popup-tab ${
							activeTab() === "participants" ? "active" : ""
						}`}
						onClick={() => setActiveTab("participants")}
					>
						{participants().length > 0
							? `${participants().length} participants`
							: "Participants"}
					</button>
				</div>
			</Show>

			<div class="popup-content">
				{/* Event Tab */}
				<div class={`tab-content ${activeTab() === "event" ? "active" : ""}`}>
					<div class="popup-form">
						<div class="popup-form-group">
							<label for="event-name-input">Event Name</label>
							<input
								id="event-name-input"
								type="text"
								placeholder="Event name"
								value={formData.name}
								onInput={(e) => handleChange("name", e.currentTarget.value)}
							/>
						</div>

						<div class="popup-form-row">
							<div class="popup-form-group">
								<label for="event-date-input">Date</label>
								<input
									id="event-date-input"
									type="date"
									value={formData.start.toISOString().split("T")[0]}
									onInput={(e) => {
										const date = new Date(e.currentTarget.value);
										handleChange("start", date);
									}}
								/>
							</div>
							<div class="popup-form-group">
								<label for="timezone-select">Timezone</label>
								<Dropdown
									selected={formData.timezone}
									onSelect={(v) => v && handleChange("timezone", v)}
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
								<label for="calendar-start-input">Start</label>
								<input
									id="calendar-start-input"
									type="time"
									value={formData.start.toTimeString().slice(0, 5)}
									onInput={(e) => {
										const [hours, minutes] = e.currentTarget.value
											.split(":")
											.map(Number);
										const newDate = new Date(formData.start);
										newDate.setHours(hours, minutes);
										handleChange("start", newDate);
									}}
								/>
							</div>
							<div class="popup-form-group">
								<label for="calendar-end-input">End</label>
								<input
									id="calendar-end-input"
									type="time"
									value={formData.end?.toTimeString().slice(0, 5)}
									onInput={(e) => {
										const [hours, minutes] = e.currentTarget.value
											.split(":")
											.map(Number);
										const newDate = formData.end ?? new Date();
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
							<label for="calendar-recurrence-input">Recurrence</label>
							<Show when={formData.recurrence}>
								{(r) => (
									<RecurrenceEditor
										startsAt={formData.start}
										recurrence={r()}
										onInput={(r) => setFormData("recurrence", r)}
									/>
								)}
							</Show>
							<Dropdown
								selected={formData.recurrence}
								onSelect={(v) => v !== null && handleChange("recurrence", v)}
								options={[
									{ item: null, label: "None" },
									{
										item: {
											frequency: "Daily",
											interval: 1,
											limit: { type: "Infinite" },
										},
										label: "Every day",
									},
									{
										item: {
											frequency: "Weekly",
											interval: 1,
											limit: { type: "Infinite" },
										},
										label: "Every week",
									},
									{
										item: {
											frequency: "Weekly",
											interval: 2,
											limit: { type: "Infinite" },
										},
										label: "Every other week",
									},
									{
										item: {
											frequency: "Monthly",
											interval: 1,
											limit: { type: "Infinite" },
										},
										label: "Every month",
									},
									{
										item: {
											frequency: "Yearly",
											interval: 1,
											limit: { type: "Infinite" },
										},
										label: "Every year",
									},
									{
										item: {
											frequency: "Daily",
											interval: 1,
											limit: { type: "Infinite" },
											by_weekday: [
												"Monday",
												"Tuesday",
												"Wednesday",
												"Thursday",
												"Friday",
											],
										},
										label: "Every weekday",
									},
								]}
							/>
						</div>

						<div class="popup-form-group">
							<label for="calendar-location-input">Location</label>
							<input
								id="calendar-location-input"
								type="text"
								placeholder="Location"
								value={formData.location}
								onInput={(e) => handleChange("location", e.currentTarget.value)}
							/>
						</div>

						<div class="popup-form-group">
							<label for="calendar-url-input">URL</label>
							<input
								id="calendar-url-input"
								type="url"
								placeholder="https://..."
								value={formData.url}
								onInput={(e) => handleChange("url", e.currentTarget.value)}
							/>
						</div>

						<div class="popup-form-group">
							<label for="calendar-description-input">Description</label>
							<textarea
								id="calendar-description-input"
								placeholder="Description"
								value={formData.description}
								onInput={(e) =>
									handleChange("description", e.currentTarget.value)
								}
							/>
						</div>

						<div class="popup-form-group">
							<label for="calendar-reminder-input">Reminder</label>
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
					{instances() ? (
						<div class="popup-form">
							<h3>Event Instances</h3>
							<p>
								This event has {instances().length} instances. Click on
								individual instances to edit them.
							</p>
						</div>
					) : (
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
					{participants() ? (
						<div class="popup-form">
							<h3>Event Participants</h3>
							<p>This event has {participants().length} participants.</p>
						</div>
					) : (
						<p>No participants added yet.</p>
					)}
				</div>
			</div>

			{/* TODO: make these buttons consistent with other buttons, maybe by copying styles do design.scss? */}
			<div class="popup-footer">
				<button
					type="button"
					class="button popup-cancel-btn"
					onClick={props.onClose}
				>
					Cancel
				</button>
				<button type="button" class="button popup-save-btn" onClick={save}>
					Save
				</button>
			</div>
		</div>
	);
};

export const RecurrenceEditor = (props: {
	recurrence: Recurrence;
	startsAt: Date;
	onInput: (recurrence: Recurrence) => void;
}) => {
	const changeFrequency = (frequency: RecurrenceFrequency) => {
		props.onInput({
			...props.recurrence,
			by_weekday: props.recurrence.by_weekday ?? [
				WEEKDAYS[props.startsAt.getDay()].full,
			],
			frequency,
		});
	};

	const changeInterval = (interval: number) => {
		props.onInput({
			...props.recurrence,
			interval,
		});
	};

	const toggleWeekday = (day: (typeof WEEKDAYS)[number]["full"]) => {
		const current = props.recurrence.by_weekday ?? [];
		const newWeekdays = current.includes(day)
			? current.filter((d) => d !== day)
			: [...current, day];
		props.onInput({
			...props.recurrence,
			by_weekday: newWeekdays,
		});
	};

	return (
		<div class="recurrence-editor">
			<div class="top row">
				Repeats every{" "}
				<input
					class="interval-input"
					type="number"
					min="1"
					placeholder="1"
					required
					value={props.recurrence.interval}
					onInput={(e) =>
						e.target.valueAsNumber && changeInterval(e.target.valueAsNumber)
					}
				/>{" "}
				<Dropdown
					selected={props.recurrence.frequency}
					onSelect={(v) => v && changeFrequency(v)}
					options={
						[
							{
								item: "Daily",
								label: props.recurrence.interval === 1 ? "day" : "days",
							},
							{
								item: "Weekly",
								label: props.recurrence.interval === 1 ? "week" : "weeks",
							},
							{
								item: "Monthly",
								label: props.recurrence.interval === 1 ? "month" : "months",
							},
							{
								item: "Yearly",
								label: props.recurrence.interval === 1 ? "year" : "years",
							},
						] as const
					}
				/>
			</div>
			<Switch>
				<Match when={props.recurrence.frequency === "Weekly"}>
					<div class="weekday-toggles row">
						On{" "}
						<For each={WEEKDAYS}>
							{(day) => (
								<button
									type="button"
									class="weekday-toggle"
									classList={{
										weekend: day.weekend,
										active:
											props.recurrence.by_weekday?.includes(day.full) ?? false,
									}}
									onClick={[toggleWeekday, day.full]}
								>
									{day.short}
								</button>
							)}
						</For>
					</div>
				</Match>
				<Match when={props.recurrence.frequency === "Monthly"}>
					<div class="row">by_weekday by_month_day</div>
				</Match>
				<Match when={props.recurrence.frequency === "Yearly"}>
					<div class="row">by_month_day</div>
				</Match>
			</Switch>
			<div class="row">forever / (count) times / until (datetime)</div>
		</div>
	);
};
