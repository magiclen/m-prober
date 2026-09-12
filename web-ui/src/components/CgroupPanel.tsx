import { SimpleGrid, Stack, Text, TextInput } from "@mantine/core";

import { durationToSeconds, formatBinaryBytes, formatDuration } from "@/format.ts";
import type { CgroupCpu, Snapshot } from "@/types.ts";

import { Details } from "./Details.tsx";
import { Panel, Unsupported } from "./Panel.tsx";
import { UsageBar } from "./UsageBar.tsx";

function Field({ name, value }: { name: string; value: string }): React.JSX.Element {
    return (
        <div>
            <Text size="sm" c="dimmed">
                {name}
            </Text>
            <Text size="sm">{value}</Text>
        </div>
    );
}

function cpuQuota(cpu: CgroupCpu): string {
    if (cpu.quota === null) {
        return "No limit set here";
    }
    const period = cpu.period === null ? 0 : durationToSeconds(cpu.period);
    return period > 0
        ? `${(durationToSeconds(cpu.quota) / period).toFixed(2)} CPUs`
        : "Unavailable";
}

function LimitUsage({
    label,
    used,
    limit,
    bytes = false,
}: {
    label: string;
    used: number;
    limit: number | null;
    bytes?: boolean;
}): React.JSX.Element {
    const value = bytes ? formatBinaryBytes(used) : String(used);
    return limit === null ? (
        <Field name={label} value={`${value} — No limit set here`} />
    ) : (
        <UsageBar
            label={label}
            used={used}
            total={limit}
            value={`${value} / ${bytes ? formatBinaryBytes(limit) : limit}`}
        />
    );
}

export function CgroupPanel({ snapshot }: { snapshot: Snapshot }): React.JSX.Element {
    const { cgroup } = snapshot;
    const cpu = cgroup?.cpu ?? null;
    const memory = cgroup?.memory ?? null;
    const pids = cgroup?.pids ?? null;

    return (
        <Panel title="cgroup">
            <Stack gap="md">
                <Text size="sm" c="dimmed">
                    Resource usage and limits of the control group that M Prober belongs to. Limits
                    shown here are this group's settings; parent groups may impose further limits.
                </Text>
                {cgroup === null ? (
                    <Unsupported>This machine does not run mprober under cgroup v2.</Unsupported>
                ) : (
                    <>
                        <TextInput label="Control group path" value={cgroup.path} readOnly />
                        <Field
                            name="CPU quota set here"
                            value={cpu === null ? "Unavailable" : cpuQuota(cpu)}
                        />
                        {memory === null ? (
                            <Field name="Memory" value="Unavailable" />
                        ) : (
                            <LimitUsage
                                label="Memory"
                                used={memory.current}
                                limit={memory.max}
                                bytes
                            />
                        )}
                        {pids === null ? (
                            <Field name="Tasks (including threads)" value="Unavailable" />
                        ) : (
                            <LimitUsage
                                label="Tasks (including threads)"
                                used={pids.current}
                                limit={pids.max}
                            />
                        )}
                        <Details label="Control group details">
                            <Stack gap="md">
                                <Text size="sm" fw={600}>
                                    CPU cumulative counters
                                </Text>
                                {cpu === null ? (
                                    <Text size="sm">Unavailable</Text>
                                ) : (
                                    <SimpleGrid cols={{ base: 1, sm: 3 }}>
                                        <Field
                                            name="Total CPU time"
                                            value={formatDuration(cpu.usage)}
                                        />
                                        <Field
                                            name="Throttled / total periods"
                                            value={`${cpu.nr_throttled} / ${cpu.nr_periods}`}
                                        />
                                        <Field
                                            name="Total throttled time"
                                            value={formatDuration(cpu.throttled)}
                                        />
                                    </SimpleGrid>
                                )}
                                <Text size="sm" fw={600}>
                                    Memory details
                                </Text>
                                {memory === null ? (
                                    <Text size="sm">Unavailable</Text>
                                ) : (
                                    <>
                                        <SimpleGrid cols={{ base: 1, sm: 2 }}>
                                            <Field
                                                name="Peak usage"
                                                value={
                                                    memory.peak === null
                                                        ? "Unavailable"
                                                        : formatBinaryBytes(memory.peak)
                                                }
                                            />
                                            <Field
                                                name="Reclaim threshold (memory.high)"
                                                value={
                                                    memory.high === null
                                                        ? "No limit set here"
                                                        : formatBinaryBytes(memory.high)
                                                }
                                            />
                                        </SimpleGrid>
                                        {memory.swap_current === null ? (
                                            <Field name="Swap" value="Unavailable" />
                                        ) : (
                                            <LimitUsage
                                                label="Swap"
                                                used={memory.swap_current}
                                                limit={memory.swap_max}
                                                bytes
                                            />
                                        )}
                                    </>
                                )}
                            </Stack>
                        </Details>
                    </>
                )}
            </Stack>
        </Panel>
    );
}
