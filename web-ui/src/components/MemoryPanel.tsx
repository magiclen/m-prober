import { Stack } from "@mantine/core";

import { formatBinaryBytes } from "@/format.ts";
import type { Snapshot } from "@/types.ts";

import { Panel } from "./Panel.tsx";
import { UsageBar } from "./UsageBar.tsx";

export function MemoryPanel({ snapshot }: { snapshot: Snapshot }): React.JSX.Element {
    const { mem, swap } = snapshot.memory;

    const bufferCache = mem.buffers + mem.cache;

    return (
        <Panel title="Memory">
            <Stack gap="sm">
                <UsageBar
                    label="Memory"
                    total={mem.total}
                    value={`${formatBinaryBytes(mem.used)} / ${formatBinaryBytes(mem.total)}`}
                    segments={[
                        { value: mem.used, color: "red", label: "used" },
                        { value: bufferCache, color: "yellow", label: "cache" },
                    ]}
                />
                <UsageBar
                    label="Swap"
                    total={swap.total}
                    value={`${formatBinaryBytes(swap.used)} / ${formatBinaryBytes(swap.total)}`}
                    segments={[
                        { value: swap.used, color: "red", label: "used" },
                        { value: swap.cache, color: "yellow", label: "cache" },
                    ]}
                />
            </Stack>
        </Panel>
    );
}
