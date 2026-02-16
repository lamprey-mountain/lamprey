import { createSignal, onMount } from "solid-js";

type DateRange = {
	start: string;
	end: string;
};

type DateRangePickerProps = {
	initialValue?: DateRange;
	onChange?: (range: DateRange) => void;
};

export const DateRangePicker = (props: DateRangePickerProps) => {
	const [dateRange, setDateRange] = createSignal<DateRange>(
		props.initialValue || {
			start: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString().split(
				"T",
			)[0],
			end: new Date().toISOString().split("T")[0],
		},
	);

	const handleStartChange = (e: Event) => {
		const target = e.target as HTMLInputElement;
		const newRange = { ...dateRange(), start: target.value };
		setDateRange(newRange);
		props.onChange?.(newRange);
	};

	const handleEndChange = (e: Event) => {
		const target = e.target as HTMLInputElement;
		const newRange = { ...dateRange(), end: target.value };
		setDateRange(newRange);
		props.onChange?.(newRange);
	};

	return (
		<div class="date-range-picker">
			<div class="date-input-group">
				<label>From:</label>
				<input
					type="date"
					value={dateRange().start}
					onInput={handleStartChange}
				/>
			</div>
			<div class="date-input-group">
				<label>To:</label>
				<input
					type="date"
					value={dateRange().end}
					onInput={handleEndChange}
				/>
			</div>
		</div>
	);
};
