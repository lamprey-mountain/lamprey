// special chars for rendering diffs with
// uses unicode private use area (pua) chars

export const INS_START = "\uE000";
export const INS_END = "\uE001";
export const DEL_START = "\uE002";
export const DEL_END = "\uE003";

export const PUA_REGEX = /[\uE000-\uE010]/;
