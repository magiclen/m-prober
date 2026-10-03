import { Stack } from "@mantine/core";

import { formatBinaryBytes, formatPercentage } from "@/format.ts";
import type { Snapshot } from "@/types.ts";

import { Panel } from "./Panel.tsx";
import { UsageBar } from "./UsageBar.tsx";

function MemoryBar({
    label,
    used,
    usedSegment,
    cache,
    total,
}: {
    label: string;
    /** The figure shown, which is what the `free` command and the CLI show. */
    used: number;
    /** The part of `used` drawn before the cache, which leaves out a cache that `used` counts too. */
    usedSegment: number;
    cache: number;
    total: number;
}): React.JSX.Element {
    const describe = (value: number): string =>
        `${formatBinaryBytes(value)} (${total > 0 ? formatPercentage(value / total) : "N/A"})`;
    return (
        <UsageBar
            label={label}
            used={used}
            total={total}
            value={`${formatBinaryBytes(used)} / ${formatBinaryBytes(total)}`}
            text={label === "Swap" && total === 0 ? "Not configured" : undefined}
            segments={[
                {
                    label: `${label} Used`,
                    value: usedSegment,
                    description: describe(usedSegment),
                },
                {
                    label: `${label} ${label === "Mem" ? "Buffers + cache" : "Cache"}`,
                    value: cache,
                    color: "gray",
                    description: describe(cache),
                },
            ]}
        />
    );
}

export function MemoryPanel({ snapshot }: { snapshot: Snapshot }): React.JSX.Element {
    const { mem, swap } = snapshot.memory;
    // `mem.used` also counts the part of the cache that cannot be reclaimed, so the cache drawn after it is clamped to the bar.
    const cache = mem.buffers + mem.cache;
    const swapUsedSegment = Math.max(0, swap.used - swap.cache);
    return (
        <Panel title="Memory">
            <Stack gap="sm">
                <MemoryBar
                    label="Mem"
                    used={mem.used}
                    usedSegment={mem.used}
                    cache={cache}
                    total={mem.total}
                />
                <MemoryBar
                    label="Swap"
                    used={swap.used}
                    usedSegment={swapUsedSegment}
                    cache={swap.cache}
                    total={swap.total}
                />
            </Stack>
        </Panel>
    );
}
