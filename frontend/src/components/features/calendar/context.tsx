import {
	createContext,
	createSignal,
	type ParentProps,
	useContext,
} from "solid-js";

export type CalendarState = {
	date: Date;
	prevMonth(): void;
	nextMonth(): void;
	gotoToday(): void;
	setMonth(month: number, year: number): void;
};

const CalendarContext = createContext<CalendarState>();

export const createCalendar = (): CalendarState => {
	const [date, setDate] = createSignal(new Date());

	return {
		get date() {
			return date();
		},
		prevMonth() {
			setDate(new Date(date().getFullYear(), date().getMonth() - 1, 1));
		},
		nextMonth() {
			setDate(new Date(date().getFullYear(), date().getMonth() + 1, 1));
		},
		gotoToday() {
			setDate(new Date());
		},
		setMonth(month: number, year: number) {
			const d = date();
			if (d.getFullYear() === year && d.getMonth() === month) return;
			setDate(new Date(year, month, 1));
		},
		// TODO: methods for navigating to {next,prev} {day,week,year}
	};
};

export const CalendarProvider = (
	props: ParentProps<{ state: CalendarState }>,
) => {
	return (
		<CalendarContext.Provider value={props.state}>
			{props.children}
		</CalendarContext.Provider>
	);
};

export const useCalendar = (): CalendarState => {
	const context = useContext(CalendarContext);
	if (!context) {
		throw new Error("useCalendar must be used within a CalendarProvider");
	}
	return context;
};
