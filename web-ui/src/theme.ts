import { createTheme } from "@mantine/core";

/**
 * The whole page is monospaced, which is what a probe should look like and what keeps every column
 * of figures lined up. The fallbacks are the monospace faces a system already has, for the moment
 * before the web font arrives and for a browser which refuses it.
 */
// The name has to match what the package registers: its variable builds carry the `Variable` suffix.
const FONT_FAMILY =
    '"Roboto Mono Variable", ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace';

export const theme = createTheme({
    fontFamily: FONT_FAMILY,
    fontFamilyMonospace: FONT_FAMILY,
    headings: { fontFamily: FONT_FAMILY },
});
