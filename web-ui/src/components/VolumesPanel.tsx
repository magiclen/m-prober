import { Group, Stack, Text } from "@mantine/core";

import { formatDecimalBytes, formatRate } from "@/format.ts";
import type { Snapshot, VolumeWithSpeed } from "@/types.ts";

import { Panel, Unsupported } from "./Panel.tsx";
import { UsageBar } from "./UsageBar.tsx";

function Volume({ volume }: { volume: VolumeWithSpeed }): React.JSX.Element {
    const label = volume.points.length > 0 ? volume.points.join(", ") : volume.device;

    return (
        <Stack gap={4}>
            <UsageBar
                label={`${label} (${volume.device}, ${volume.fs_type})`}
                total={volume.size}
                value={`${formatDecimalBytes(volume.used)} / ${formatDecimalBytes(volume.size)}`}
                segments={[{ value: volume.used, color: "red", label: "" }]}
            />
            <Group gap="md">
                <Text size="xs" c="dimmed">
                    Read {formatRate(volume.speed.read)}
                </Text>
                <Text size="xs" c="dimmed">
                    Write {formatRate(volume.speed.write)}
                </Text>
            </Group>
        </Stack>
    );
}

export function VolumesPanel({ snapshot }: { snapshot: Snapshot }): React.JSX.Element {
    if (snapshot.volumes.length === 0) {
        return (
            <Panel title="Volumes">
                <Unsupported>No mounted volume was found.</Unsupported>
            </Panel>
        );
    }

    return (
        <Panel title="Volumes">
            <Stack gap="sm">
                {snapshot.volumes.map((volume) => (
                    <Volume key={volume.device} volume={volume} />
                ))}
            </Stack>
        </Panel>
    );
}
