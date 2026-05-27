import { VoidProps } from "solid-js";

export type ChartProps = {
  points: Array<number>; height: number; unit?: string

  // data?: Array<{ bucket: string } & T>;
  // field: keyof T;
  // name: string;
  // formatter?: (value: number) => string;
  // hoveredTime: string | null;
  // setHoveredTime: (time: string | null) => void;
  // selectionStartTime: string | null;
  // setSelectionStartTime: (time: string | null) => void;
  // onZoom: (startBucket: string, endBucket: string) => void;

}

export const Chart = (
  props: VoidProps<ChartProps>,
) => {
  // TODO: copy chart impl here
  // use in room analytics, voice stats
}
