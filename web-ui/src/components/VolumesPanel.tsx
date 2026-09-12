import { Table, Text, Tooltip } from "@mantine/core";

import { formatDecimalBytes, formatPercentage, formatRate } from "@/format.ts";
import type { Snapshot } from "@/types.ts";

import { Panel, Unsupported } from "./Panel.tsx";
import { UsageMeter } from "./UsageMeter.tsx";

import classes from "./VolumesPanel.module.css";

export function VolumesPanel({ snapshot }: { snapshot: Snapshot }): React.JSX.Element {
    return (
        <Panel title="Volumes">
            {snapshot.volumes.length === 0 ? (
                <Unsupported>No mounted volume was found.</Unsupported>
            ) : (
                <Table.ScrollContainer minWidth={780}>
                    <Table className={classes.table} withRowBorders={false} verticalSpacing="xs">
                        <Table.Thead>
                            <Table.Tr>
                                <Table.Th aria-label="Device" />
                                <Table.Th>Reading Rate</Table.Th>
                                <Table.Th>Read Data</Table.Th>
                                <Table.Th>Writing Rate</Table.Th>
                                <Table.Th>Written Data</Table.Th>
                                <Table.Th>Mount Points</Table.Th>
                            </Table.Tr>
                        </Table.Thead>
                        {snapshot.volumes.map((volume) => (
                            <Table.Tbody key={volume.device}>
                                <Table.Tr>
                                    <Table.Th scope="row">
                                        <Tooltip
                                            label={volume.fs_type}
                                            events={{ hover: true, focus: true, touch: false }}
                                            withArrow
                                        >
                                            <Text
                                                component="span"
                                                size="sm"
                                                fw={600}
                                                c="cyan"
                                                tabIndex={0}
                                            >
                                                {volume.device}
                                            </Text>
                                        </Tooltip>
                                    </Table.Th>
                                    <Table.Td>{formatRate(volume.speed.read)}</Table.Td>
                                    <Table.Td>
                                        {formatDecimalBytes(volume.stat.read_bytes)}
                                    </Table.Td>
                                    <Table.Td>{formatRate(volume.speed.write)}</Table.Td>
                                    <Table.Td>
                                        {formatDecimalBytes(volume.stat.write_bytes)}
                                    </Table.Td>
                                    <Table.Td>{volume.points[0] ?? ""}</Table.Td>
                                </Table.Tr>
                                <Table.Tr>
                                    <Table.Td />
                                    <Table.Td colSpan={2}>
                                        <UsageMeter
                                            label={`${volume.device} usage`}
                                            used={volume.used}
                                            total={volume.size}
                                        />
                                    </Table.Td>
                                    <Table.Td colSpan={2}>
                                        {formatDecimalBytes(volume.used)} /{" "}
                                        {formatDecimalBytes(volume.size)} (
                                        {volume.size > 0
                                            ? formatPercentage(volume.used / volume.size)
                                            : "N/A"}
                                        )
                                    </Table.Td>
                                    <Table.Td>{volume.points[1] ?? ""}</Table.Td>
                                </Table.Tr>
                                {volume.points.slice(2).map((point) => (
                                    <Table.Tr key={point}>
                                        <Table.Td colSpan={5} />
                                        <Table.Td>{point}</Table.Td>
                                    </Table.Tr>
                                ))}
                            </Table.Tbody>
                        ))}
                    </Table>
                </Table.ScrollContainer>
            )}
        </Panel>
    );
}
