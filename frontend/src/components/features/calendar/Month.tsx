import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { Key } from "@solid-primitives/keyed";
import { ReactiveSet } from "@solid-primitives/set";
import {
	createEffect,
	createMemo,
	For,
	on,
	onCleanup,
	onMount,
} from "solid-js";
import type { CalendarEvent, Channel } from "ts-sdk";
import { useCtx } from "@/app/context";
import { useCalendar } from "./context";

export const CalendarMonth = (props: {
	channel: Channel;
	events: Array<CalendarEvent>;
	onDayClick: (day: number, el: HTMLElement) => void;
	onEventClick: (event: CalendarEvent, day: number, el: HTMLElement) => void;
}) => {
	const ctx = useCtx();
	const calendar = useCalendar();

	const daysPerWeek = () => 7;

	const dayStartsAt = createMemo(() => {
		const dsa = ctx.preferences().frontend.calendar_day_starts_at;
		return typeof dsa === "number" ? dsa : 0;
	});

	const displayDaysOfWeek = createMemo(() => {
		const daysOfWeek = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
		const start = dayStartsAt();
		return [...daysOfWeek.slice(start), ...daysOfWeek.slice(0, start)];
	});

	const OVERSCAN = 2;

	const calendarDays = createMemo(() => {
		const days = [];
		const baseDate = new Date(calendar.date);
		const currentMonth = baseDate.getMonth();
		const currentYear = baseDate.getFullYear();

		for (let m = -OVERSCAN; m <= OVERSCAN; m++) {
			const start = new Date(currentYear, currentMonth + m, 1);
			const year = start.getFullYear();
			const month = start.getMonth();
			const daysInMonth = new Date(year, month + 1, 0).getDate();
			const firstDayWeekday =
				(start.getDay() - dayStartsAt() + daysPerWeek()) % daysPerWeek();

			const daysInPrevMonth = new Date(year, month, 0).getDate();

			if (m === -OVERSCAN) {
				for (let i = firstDayWeekday; i > 0; i--) {
					days.push({
						day: daysInPrevMonth - i + 1,
						month: month - 1,
						year: year,
						isOtherMonth: true,
					});
				}
			}

			for (let i = 1; i <= daysInMonth; i++) {
				days.push({
					day: i,
					month,
					year,
					isOtherMonth: month !== currentMonth,
				});
			}
		}

		return days;
	});

	// TODO: make today and isToday reactive, automatically update when the day changes
	const today = new Date();
	const isToday = (day: number, month: number, year: number) => {
		return (
			day === today.getDate() &&
			month === today.getMonth() &&
			year === today.getFullYear()
		);
	};

	let monthGridRef!: HTMLDivElement;
	const monthStartRefs = new ReactiveSet<HTMLElement>();

	const scrollToCurrentMonth = () => {
		const year = calendar.date.getFullYear();
		const month = calendar.date.getMonth();
		const ref = monthGridRef.querySelector(
			`[data-year="${year}"][data-month="${month}"]`,
		);
		if (!ref) return;

		ref.scrollIntoView({ behavior: "instant", block: "start" });
	};

	createEffect(
		on(
			() => calendar.date,
			() => setTimeout(scrollToCurrentMonth),
		),
	);

	onMount(() => {
		scrollToCurrentMonth();

		createIntersectionObserver(
			() => [...monthStartRefs],
			(entries) => {
				for (const entry of entries) {
					if (!entry.isIntersecting) continue;
					const el = entry.target as HTMLElement;
					const month = Number.parseInt(el.dataset.month ?? "0", 10);
					const year = Number.parseInt(el.dataset.year ?? "0", 10);
					calendar.setMonth(month, year);
				}
			},
			{
				root: monthGridRef,
				rootMargin: "0px 0px -50% 0px",
				threshold: 0,
			},
		);
	});

	// FIXME: very jumpy scrolling when navigating through months

	return (
		<div
			class="month-view"
			style={{
				"--days-per-week": daysPerWeek(),
				"--row-count": Math.ceil(calendarDays().length / daysPerWeek()),
			}}
		>
			<div class="days-of-week">
				<For each={displayDaysOfWeek()}>{(i) => <div>{i}</div>}</For>
			</div>
			<div class="month" ref={monthGridRef}>
				<Key each={calendarDays()} by={(d) => `${d.year}-${d.month}-${d.day}`}>
					{(d) => (
						<div
							class="day"
							classList={{
								"month-start": d().day === 1,
								othermonth: d().isOtherMonth,
								today:
									!d().isOtherMonth && isToday(d().day, d().month, d().year),
							}}
							ref={(el) => {
								if (d().day === 1) {
									monthStartRefs.add(el);
									onCleanup(() => monthStartRefs.delete(el));
								}
							}}
							onClick={(e) =>
								props.onDayClick(d().day, e.target as HTMLElement)
							}
							data-month={d().month}
							data-year={d().year}
						>
							<div class="daynumber">{d().day}</div>
							<For
								// PERF: dont filter every event for every day
								each={props.events.filter((e) => {
									const d_start = new Date(e.starts_at);
									return (
										d_start.getDate() === d().day &&
										d_start.getMonth() === d().month &&
										d_start.getFullYear() === d().year
									);
								})}
							>
								{(event) => (
									<div
										class="event"
										ref={(el) => {
											if (el) {
												el.addEventListener("click", (e) => {
													e.stopPropagation();
													props.onEventClick(event, d().day, el);
												});
											}
										}}
									>
										{event.title}
									</div>
								)}
							</For>
						</div>
					)}
				</Key>
			</div>
		</div>
	);
};
