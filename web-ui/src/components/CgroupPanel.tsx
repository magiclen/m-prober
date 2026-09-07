import { SimpleGrid, Stack, Text } from "@mantine/core";

import { durationToSeconds, formatBinaryBytes, formatDuration } from "@/format.ts";
import type { CgroupCpu, Snapshot } from "@/types.ts";

import { Panel, Unsupported } from "./Panel.tsx";
import { UsageBar } from "./UsageBar.tsx";

function Field({ name, value }: { name: string; value: string }): React.JSX.Element {
    return (
        <div>
            <Text size="xs" c="dimmed" tt="uppercase">
                {name}
            </Text>
            <Text size="sm" fw={500}>
                {value}
            </Text>
        </div>
    );
}

/** `cpu.max` gives a quota per period, whose ratio is how many whole CPUs the cgroup may use. */
function effectiveCpuCount(cpu: CgroupCpu): number | null {
    if (cpu.quota === null || cpu.period === null) {
        return null;
    }

    const period = durationToSeconds(cpu.period);

    return period > 0 ? durationToSeconds(cpu.quota) / period : null;
}

export function CgroupPanel({ snapshot }: { snapshot: Snapshot }): React.JSX.Element {
    const { cgroup } = snapshot;

    if (cgroup === null) {
        return (
            <Panel title="cgroup">
                <Unsupported>This machine does not run mprober under cgroup v2.</Unsupported>
            </Panel>
        );
    }

    const { cpu, memory, pids } = cgroup;
    const cpuCount = cpu === null ? null : effectiveCpuCount(cpu);

    return (
        <Panel title="cgroup" aside={cgroup.path}>
            <Stack gap="sm">
                {cpu !== null && (
                    <SimpleGrid cols={{ base: 2, sm: 4 }} spacing="sm">
                        <Field
                            name="CPU limit"
                            value={
                                cpuCount === null ? "not limited" : `${cpuCount.toFixed(2)} CPUs`
                            }
                        />
                        <Field name="CPU used" value={formatDuration(cpu.usage)} />
                        <Field
                            name="Throttled"
                            value={`${cpu.nr_throttled} / ${cpu.nr_periods} periods`}
                        />
                        <Field name="Throttled for" value={formatDuration(cpu.throttled)} />
                    </SimpleGrid>
                )}

                {memory !== null &&
                    (memory.max === null ? (
                        <Text size="sm">
                            Memory {formatBinaryBytes(memory.current)}, not limited
                        </Text>
                    ) : (
                        <UsageBar
                            label="Memory"
                            total={memory.max}
                            value={`${formatBinaryBytes(memory.current)} / ${formatBinaryBytes(
                                memory.max,
                            )}`}
                            segments={[{ value: memory.current, color: "red", label: "" }]}
                        />
                    ))}

                {pids !== null &&
                    (pids.max === null ? (
                        <Text size="sm">PIDs {pids.current}, not limited</Text>
                    ) : (
                        <UsageBar
                            label="PIDs"
                            total={pids.max}
                            value={`${pids.current} / ${pids.max}`}
                            segments={[{ value: pids.current, color: "grape", label: "" }]}
                        />
                    ))}
            </Stack>
        </Panel>
    );
}
