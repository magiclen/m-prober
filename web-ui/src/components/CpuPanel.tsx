import { Stack, Text } from "@mantine/core";

import { formatFrequency, formatPercentage } from "@/format.ts";
import type { Snapshot } from "@/types.ts";

import { Panel } from "./Panel.tsx";
import { UsageBar } from "./UsageBar.tsx";

import classes from "./CpuPanel.module.css";

const UNKNOWN_MODEL_NAME = "Unknown CPU";

function usageColor(ratio: number): string {
    if (ratio >= 0.9) {
        return "red";
    }

    return ratio >= 0.7 ? "orange" : "cyan";
}

function CoreBar({ index, ratio }: { index: number; ratio: number }): React.JSX.Element {
    return (
        <UsageBar
            label={`Core ${index}`}
            total={1}
            value={formatPercentage(ratio)}
            segments={[{ value: ratio, color: usageColor(ratio), label: "" }]}
        />
    );
}

export function CpuPanel({ snapshot }: { snapshot: Snapshot }): React.JSX.Element {
    const { load_average: load, cpus, cpus_stat: stat } = snapshot;

    // The first entry is the average over every CPU and the rest are the individual ones.
    const average = stat[0] ?? 0;
    const cores = stat.slice(1).map((ratio, index) => ({ core: index, ratio }));

    const logicalCores = cpus.reduce((sum, cpu) => sum + cpu.siblings, 0);

    const models = cpus.map((cpu) => {
        const averageMhz =
            cpu.cpus_mhz.length > 0
                ? cpu.cpus_mhz.reduce((sum, mhz) => sum + mhz, 0) / cpu.cpus_mhz.length
                : 0;

        return `${cpu.model_name ?? UNKNOWN_MODEL_NAME} ${cpu.cpu_cores}C/${cpu.siblings}T @ ${formatFrequency(
            averageMhz,
        )}`;
    });

    return (
        <Panel title="CPU" aside={models.join(" / ")}>
            <Stack gap="sm">
                {/* The total is one value, so its bar is kept to a width the eye can cross rather
                    than stretched across a wide screen with the label and the value at either end. */}
                <div className={classes.summary}>
                    <UsageBar
                        label="Total"
                        total={1}
                        value={formatPercentage(average)}
                        segments={[{ value: average, color: usageColor(average), label: "" }]}
                    />

                    <Text size="sm" c="dimmed">
                        Load average {load.one.toFixed(2)} / {load.five.toFixed(2)} /{" "}
                        {load.fifteen.toFixed(2)} over {logicalCores} logical cores
                    </Text>
                </div>

                <div className={classes.cores}>
                    {cores.map(({ core, ratio }) => (
                        <CoreBar key={core} index={core} ratio={ratio} />
                    ))}
                </div>
            </Stack>
        </Panel>
    );
}
