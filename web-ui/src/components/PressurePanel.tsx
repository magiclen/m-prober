import { Table, Text } from "@mantine/core";

import { formatDuration } from "@/format.ts";
import type { Pressure, PressureStat, Snapshot } from "@/types.ts";

import { Panel, Unsupported } from "./Panel.tsx";

import classes from "./DataTable.module.css";

interface Row {
    resource: string;
    kind: string;
    stat: PressureStat;
}

function collect(resources: [string, Pressure][]): Row[] {
    return resources.flatMap(([resource, pressure]) => {
        const rows: Row[] = [{ resource, kind: "some", stat: pressure.some }];

        if (pressure.full !== null) {
            rows.push({ resource, kind: "full", stat: pressure.full });
        }

        return rows;
    });
}

export function PressurePanel({ snapshot }: { snapshot: Snapshot }): React.JSX.Element {
    const { pressure } = snapshot;

    if (pressure === null) {
        return (
            <Panel title="Pressure (PSI)">
                <Unsupported>
                    This kernel does not provide PSI. It needs CONFIG_PSI and must not be booted
                    with psi=0.
                </Unsupported>
            </Panel>
        );
    }

    const rows = collect([
        ["CPU", pressure.cpu],
        ["Memory", pressure.memory],
        ["IO", pressure.io],
    ]);

    return (
        <Panel title="Pressure (PSI)" aside="Share of time stalled waiting for the resource">
            <Table.ScrollContainer minWidth={420}>
                <Table striped className={classes.table}>
                    <Table.Thead>
                        <Table.Tr>
                            <Table.Th>Resource</Table.Th>
                            <Table.Th ta="right">avg10</Table.Th>
                            <Table.Th ta="right">avg60</Table.Th>
                            <Table.Th ta="right">avg300</Table.Th>
                            <Table.Th ta="right">Total</Table.Th>
                        </Table.Tr>
                    </Table.Thead>
                    <Table.Tbody>
                        {rows.map(({ resource, kind, stat }) => (
                            <Table.Tr key={`${resource}-${kind}`}>
                                <Table.Td>
                                    <Text size="sm" fw={500}>
                                        {resource} {kind}
                                    </Text>
                                </Table.Td>
                                <Table.Td ta="right">{stat.avg10.toFixed(2)}%</Table.Td>
                                <Table.Td ta="right">{stat.avg60.toFixed(2)}%</Table.Td>
                                <Table.Td ta="right">{stat.avg300.toFixed(2)}%</Table.Td>
                                <Table.Td ta="right">{formatDuration(stat.total)}</Table.Td>
                            </Table.Tr>
                        ))}
                    </Table.Tbody>
                </Table>
            </Table.ScrollContainer>
        </Panel>
    );
}
