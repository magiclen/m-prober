import type { Duration } from "@/types.ts";

const BINARY_UNITS = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
const DECIMAL_UNITS = ["B", "KB", "MB", "GB", "TB", "PB"];

const scale = (value: number, base: number, units: string[]): string => {
    let index = 0;

    while (Math.abs(value) >= base && index < units.length - 1) {
        value /= base;
        index += 1;
    }

    // Whole bytes are never shown with decimals, since `1.00 B` reads worse than `1 B`.
    const digits = index === 0 ? 0 : 2;

    return `${value.toFixed(digits)} ${units[index]}`;
};

/** Memory is what the kernel allocates in powers of two, so it is shown in binary units. */
export const formatBinaryBytes = (value: number): string => scale(value, 1024, BINARY_UNITS);

/** Storage and traffic are what vendors count in powers of ten. */
export const formatDecimalBytes = (value: number): string => scale(value, 1000, DECIMAL_UNITS);

export const formatRate = (bytesPerSecond: number): string =>
    `${formatDecimalBytes(bytesPerSecond)}/s`;

export const formatPercentage = (ratio: number): string => `${(ratio * 100).toFixed(2)}%`;

export const durationToSeconds = (duration: Duration): number =>
    duration.secs + duration.nanos / 1_000_000_000;

export const formatDuration = (duration: Duration): string => {
    let seconds = Math.floor(durationToSeconds(duration));

    const days = Math.floor(seconds / 86400);
    seconds %= 86400;

    const hours = Math.floor(seconds / 3600);
    seconds %= 3600;

    const minutes = Math.floor(seconds / 60);
    seconds %= 60;

    const parts: string[] = [];

    if (days > 0) {
        parts.push(`${days}d`);
    }

    if (days > 0 || hours > 0) {
        parts.push(`${hours}h`);
    }

    if (days > 0 || hours > 0 || minutes > 0) {
        parts.push(`${minutes}m`);
    }

    parts.push(`${seconds}s`);

    return parts.join(" ");
};

/** `/proc/cpuinfo` reports the frequency in MHz, which is worth scaling once it reaches GHz. */
export const formatFrequency = (mhz: number): string =>
    mhz >= 1000 ? `${(mhz / 1000).toFixed(2)}GHz` : `${mhz.toFixed(2)}MHz`;

export const formatDateTime = (rtcTime: string): string => rtcTime.replace("T", " ");
