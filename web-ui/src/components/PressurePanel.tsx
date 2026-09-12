import { Stack, Table, Text } from "@mantine/core";

import { formatDuration } from "@/format.ts";
import type { Pressure, Snapshot } from "@/types.ts";

import { Details } from "./Details.tsx";
import { Panel, Unsupported } from "./Panel.tsx";
import { UsageBar } from "./UsageBar.tsx";

import classes from "./DataTable.module.css";

export function PressurePanel({ snapshot }: { snapshot: Snapshot }): React.JSX.Element {
    const { pressure } = snapshot;
    const resources: [string, Pressure][] =
        pressure === null
            ? []
            : [
                  ["CPU", pressure.cpu],
                  ["Memory", pressure.memory],
                  ["I/O", pressure.io],
              ];

    return (
        <Panel title="Pressure (PSI)">
            <Stack gap="md">
                <Text size="sm" c="dimmed">
                    Pressure Stall Information shows time that tasks spend waiting for a resource,
                    not resource utilization. Lower values mean less waiting.
                </Text>
                {pressure === null ? (
                    <Unsupported>This kernel does not provide PSI.</Unsupported>
                ) : (
                    <>
                        <Text size="sm">Some tasks waiting — last 10 seconds</Text>
                        {resources.map(([name, resource]) => (
                            <UsageBar
                                key={name}
                                label={name}
                                used={resource.some.avg10}
                                total={100}
                                color="gray"
                            />
                        ))}
                        <Details label="Pressure details">
                            <Stack gap="md">
                                {resources.map(([name, resource]) => (
                                    <div key={name}>
                                        <Text fw={600} size="sm" mb="xs">
                                            {name}
                                        </Text>
                                        <Table.ScrollContainer minWidth={720}>
                                            <Table className={classes.table} striped>
                                                <Table.Thead>
                                                    <Table.Tr>
                                                        <Table.Th>Waiting tasks</Table.Th>
                                                        <Table.Th ta="right">10 seconds</Table.Th>
                                                        <Table.Th ta="right">1 minute</Table.Th>
                                                        <Table.Th ta="right">5 minutes</Table.Th>
                                                        <Table.Th ta="right">
                                                            Total stall time
                                                        </Table.Th>
                                                    </Table.Tr>
                                                </Table.Thead>
                                                <Table.Tbody>
                                                    {(["some", "full"] as const).map((kind) => {
                                                        const stat = resource[kind];
                                                        return (
                                                            <Table.Tr key={kind}>
                                                                <Table.Th scope="row">
                                                                    {kind === "some"
                                                                        ? "Some tasks waiting (some)"
                                                                        : "All non-idle tasks waiting (full)"}
                                                                </Table.Th>
                                                                {stat === null ? (
                                                                    <Table.Td colSpan={4}>
                                                                        Unavailable
                                                                    </Table.Td>
                                                                ) : (
                                                                    <>
                                                                        <Table.Td ta="right">
                                                                            {stat.avg10.toFixed(2)}%
                                                                        </Table.Td>
                                                                        <Table.Td ta="right">
                                                                            {stat.avg60.toFixed(2)}%
                                                                        </Table.Td>
                                                                        <Table.Td ta="right">
                                                                            {stat.avg300.toFixed(2)}
                                                                            %
                                                                        </Table.Td>
                                                                        <Table.Td ta="right">
                                                                            {formatDuration(
                                                                                stat.total,
                                                                            )}
                                                                        </Table.Td>
                                                                    </>
                                                                )}
                                                            </Table.Tr>
                                                        );
                                                    })}
                                                </Table.Tbody>
                                            </Table>
                                        </Table.ScrollContainer>
                                    </div>
                                ))}
                            </Stack>
                        </Details>
                    </>
                )}
            </Stack>
        </Panel>
    );
}
