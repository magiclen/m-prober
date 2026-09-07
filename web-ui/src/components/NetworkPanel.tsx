import { Table, Text } from "@mantine/core";

import { formatDecimalBytes, formatRate } from "@/format.ts";
import type { Snapshot } from "@/types.ts";

import { Panel, Unsupported } from "./Panel.tsx";

export function NetworkPanel({ snapshot }: { snapshot: Snapshot }): React.JSX.Element {
    if (snapshot.network.length === 0) {
        return (
            <Panel title="Network">
                <Unsupported>No network interface was found.</Unsupported>
            </Panel>
        );
    }

    return (
        <Panel title="Network">
            <Table.ScrollContainer minWidth={480}>
                <Table striped highlightOnHover>
                    <Table.Thead>
                        <Table.Tr>
                            <Table.Th>Interface</Table.Th>
                            <Table.Th ta="right">Download</Table.Th>
                            <Table.Th ta="right">Upload</Table.Th>
                            <Table.Th ta="right">Received</Table.Th>
                            <Table.Th ta="right">Transmitted</Table.Th>
                        </Table.Tr>
                    </Table.Thead>
                    <Table.Tbody>
                        {snapshot.network.map((network) => (
                            <Table.Tr key={network.interface}>
                                <Table.Td>
                                    <Text size="sm" fw={500}>
                                        {network.interface}
                                    </Text>
                                </Table.Td>
                                <Table.Td ta="right">{formatRate(network.speed.receive)}</Table.Td>
                                <Table.Td ta="right">{formatRate(network.speed.transmit)}</Table.Td>
                                <Table.Td ta="right">
                                    {formatDecimalBytes(network.stat.receive_bytes)}
                                </Table.Td>
                                <Table.Td ta="right">
                                    {formatDecimalBytes(network.stat.transmit_bytes)}
                                </Table.Td>
                            </Table.Tr>
                        ))}
                    </Table.Tbody>
                </Table>
            </Table.ScrollContainer>
        </Panel>
    );
}
