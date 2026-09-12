import { Stack, Text } from "@mantine/core";

import type { Snapshot } from "@/types.ts";

import { Panel } from "./Panel.tsx";
import { UsageBar } from "./UsageBar.tsx";

export function LoadPanel({ snapshot }: { snapshot: Snapshot }): React.JSX.Element {
    const logicalCores = snapshot.cpus.reduce((sum, cpu) => sum + cpu.siblings, 0);
    const load = snapshot.load_average;

    return (
        <Panel title="Load Average">
            <Stack gap="sm">
                <Text size="sm" c="dimmed">
                    Load relative to {logicalCores} logical CPU cores. This is not CPU utilization.
                </Text>
                <UsageBar
                    label="1 minute"
                    used={load.one}
                    total={logicalCores}
                    value={load.one.toFixed(2)}
                />
                <UsageBar
                    label="5 minutes"
                    used={load.five}
                    total={logicalCores}
                    value={load.five.toFixed(2)}
                />
                <UsageBar
                    label="15 minutes"
                    used={load.fifteen}
                    total={logicalCores}
                    value={load.fifteen.toFixed(2)}
                />
            </Stack>
        </Panel>
    );
}
