import { describe, expect, test } from "vitest";

import {
    durationToSeconds,
    formatBinaryBytes,
    formatDecimalBytes,
    formatDuration,
    formatFrequency,
    formatPercentage,
    formatRate,
} from "@/format.ts";

describe("formatBinaryBytes", () => {
    test("keeps whole bytes without decimals", () => {
        expect("0 B").toBe(formatBinaryBytes(0));
        expect("512 B").toBe(formatBinaryBytes(512));
    });

    test("scales by 1024", () => {
        expect("1.00 KiB").toBe(formatBinaryBytes(1024));
        expect("1.00 GiB").toBe(formatBinaryBytes(1024 ** 3));
        expect("1.50 GiB").toBe(formatBinaryBytes(1024 ** 3 * 1.5));
    });
});

describe("formatDecimalBytes", () => {
    test("scales by 1000", () => {
        expect("1.00 KB").toBe(formatDecimalBytes(1000));
        expect("2.50 MB").toBe(formatDecimalBytes(2_500_000));
    });
});

describe("formatRate", () => {
    test("appends a per-second suffix", () => {
        expect("1.00 MB/s").toBe(formatRate(1_000_000));
    });
});

describe("formatPercentage", () => {
    test("turns a ratio into a percentage", () => {
        expect("0.00%").toBe(formatPercentage(0));
        expect("12.34%").toBe(formatPercentage(0.1234));
        expect("100.00%").toBe(formatPercentage(1));
    });
});

describe("durationToSeconds", () => {
    test("folds the nanoseconds in", () => {
        expect(1.5).toBe(durationToSeconds({ secs: 1, nanos: 500_000_000 }));
    });
});

describe("formatDuration", () => {
    test("shows only the parts which are reached", () => {
        expect("0s").toBe(formatDuration({ secs: 0, nanos: 0 }));
        expect("42s").toBe(formatDuration({ secs: 42, nanos: 0 }));
        expect("2m 3s").toBe(formatDuration({ secs: 123, nanos: 0 }));
        expect("1h 0m 0s").toBe(formatDuration({ secs: 3600, nanos: 0 }));
        expect("2d 3h 4m 5s").toBe(formatDuration({ secs: 183_845, nanos: 0 }));
    });
});

describe("formatFrequency", () => {
    test("switches to GHz at 1000 MHz", () => {
        expect("800.00MHz").toBe(formatFrequency(800));
        expect("3.60GHz").toBe(formatFrequency(3600));
    });
});
