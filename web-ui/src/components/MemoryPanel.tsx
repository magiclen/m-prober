import { Stack } from "@mantine/core";

import { formatBinaryBytes, formatPercentage } from "@/format.ts";
import type { Snapshot } from "@/types.ts";

import { Panel } from "./Panel.tsx";
import { UsageBar } from "./UsageBar.tsx";

function MemoryBar({
    label,
    used,
    cache,
    total,
}: {
    label: string;
    used: number;
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
                { label: `${label} Used`, value: used, description: describe(used) },
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
    const cache = mem.buffers + mem.cache;
    // The legacy display counts cache separately from used memory.
    const memUsed = Math.max(0, mem.total - mem.free - cache);
    const swapUsed = Math.max(0, swap.used - swap.cache);
    return (
        <Panel title="Memory">
            <Stack gap="sm">
                <MemoryBar label="Mem" used={memUsed} cache={cache} total={mem.total} />
                <MemoryBar label="Swap" used={swapUsed} cache={swap.cache} total={swap.total} />
            </Stack>
        </Panel>
    );
}
