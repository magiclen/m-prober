import { Stack, Text } from "@mantine/core";

import { formatFrequency, formatPercentage } from "@/format.ts";
import type { CpuThreadSnapshot, Snapshot } from "@/types.ts";

import { Details } from "./Details.tsx";
import { Panel } from "./Panel.tsx";
import { UsageBar } from "./UsageBar.tsx";

function ThreadRows({ threads }: { threads: CpuThreadSnapshot[] }): React.JSX.Element {
    return (
        <Stack gap="sm">
            {threads.length === 0 ? (
                <Text size="sm">Unavailable</Text>
            ) : (
                threads.map((thread) => (
                    <UsageBar
                        key={thread.id}
                        label={`CPU${thread.id}`}
                        used={thread.usage ?? 0}
                        total={thread.usage === null ? 0 : 1}
                        text={`${thread.usage === null ? "Unavailable" : formatPercentage(thread.usage)} (${thread.frequency_mhz === null ? "Unavailable" : formatFrequency(thread.frequency_mhz)})`}
                    />
                ))
            )}
        </Stack>
    );
}

export function CpuPanel({ snapshot }: { snapshot: Snapshot }): React.JSX.Element {
    const { cpus, cpus_stat: stat } = snapshot;
    const threads = snapshot.cpu_threads.toSorted((a, b) => a.id - b.id);
    const knownIds = new Set(cpus.map((cpu) => cpu.physical_id));
    const unknown = threads.filter(
        (thread) => thread.physical_id === null || !knownIds.has(thread.physical_id),
    );
    return (
        <Panel title="CPU">
            <Stack gap="md">
                <UsageBar label="CPU" used={stat[0] ?? 0} total={stat.length > 0 ? 1 : 0} />
                {cpus.map((cpu) => {
                    const members = threads.filter(
                        (thread) => thread.physical_id === cpu.physical_id,
                    );
                    const frequencies = members.flatMap((thread) =>
                        thread.frequency_mhz === null ? [] : [thread.frequency_mhz],
                    );
                    const average =
                        frequencies.length === 0
                            ? "Unavailable"
                            : formatFrequency(
                                  frequencies.reduce((sum, value) => sum + value, 0) /
                                      frequencies.length,
                              );
                    return (
                        <Details
                            key={cpu.physical_id}
                            label={`${cpu.model_name ?? "Unknown CPU"} ${cpu.cpu_cores}C/${cpu.siblings}T ${average}`}
                        >
                            <ThreadRows threads={members} />
                        </Details>
                    );
                })}
                {unknown.length > 0 && (
                    <Details label="Unknown CPU">
                        <ThreadRows threads={unknown} />
                    </Details>
                )}
            </Stack>
        </Panel>
    );
}
