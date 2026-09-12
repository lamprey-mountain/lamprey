import type { CalendarEventCreate, CalendarEventPatch } from "sdk/types";
import {
	createContext,
	createEffect,
	createSignal,
	type ParentProps,
	Show,
	useContext,
} from "solid-js";
import { createStore } from "solid-js/store";
import { useApi } from "@/api";
import { CheckboxOption } from "@/atoms/CheckboxOption";
import { Dropdown } from "@/atoms/Dropdown";
import { Checkbox, XMark } from "@/atoms/icons";

export type CalendarPopup = {
	ref: HTMLElement | null;
	id: "event-editor";
	props: {
		channel_id: string;
		event?: {
			id?: string;
			name: string;
			start: Date;
			end: Date | null;
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
		end: Date | null;
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
			end: event.end ?? undefined, // NOTE: check if undefined works
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
					location: formData.location || null,
					url: formData.url || null,
					starts_at: formData.start.toISOString(),
					ends_at: formData.end.toISOString(), // TODO: make optional
				} as CalendarEventPatch,
			});
		} else {
			api.client.http.POST("/api/v1/calendar/{channel_id}/event", {
				params: { path: { channel_id: props.channel_id } },
				body: {
					title: formData.name, // TODO: require
					description: formData.description || null,
					location: formData.location || null,
					recurrence: null, // TODO: better recurrence input
					timezone: null, // TODO: better timezone input
					url: formData.url || null,
					starts_at: formData.start.toISOString(),
					ends_at: formData.end.toISOString(), // TODO: make optional
				} as CalendarEventCreate,
			});
		}
	};

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
							{props.event?.instances && props.event.instances.length > 0
								? `${props.event.instances.length} instances`
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
									value={formData.end.toTimeString().slice(0, 5)}
									onInput={(e) => {
										const [hours, minutes] = e.currentTarget.value
											.split(":")
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
							<label for="calendar-recurrence-input">Recurrence</label>
							<Dropdown
								selected={formData.recurrence}
								onSelect={(v) => v !== null && handleChange("recurrence", v)}
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
					{props.event?.instances ? (
						<div class="popup-form">
							<h3>Event Instances</h3>
							<p>
								This event has {props.event.instances.length} instances. Click
								on individual instances to edit them.
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
					{props.event?.participants ? (
						<div class="popup-form">
							<h3>Event Participants</h3>
							<p>
								This event has {props.event.participants.length} participants.
							</p>
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
