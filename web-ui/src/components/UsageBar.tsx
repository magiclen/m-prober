import { Group, Progress, Text } from "@mantine/core";

import { formatPercentage } from "@/format.ts";

interface Segment {
    value: number;
    color: string;
    label: string;
}

interface UsageBarProps {
    label: string;
    /** The segments are drawn in order and are expected to add up to at most `total`. */
    segments: Segment[];
    total: number;
    /** Shown at the right, in place of the percentage when given. */
    value?: string;
}

export function UsageBar({ label, segments, total, value }: UsageBarProps): React.JSX.Element {
    const used = segments.reduce((sum, segment) => sum + segment.value, 0);

    // A total of zero happens for a machine without swap, and must not turn every section into `NaN%`.
    const ratio = total > 0 ? used / total : 0;

    return (
        <div>
            <Group justify="space-between" gap="xs" wrap="nowrap" mb={4}>
                <Text size="sm" fw={500} truncate>
                    {label}
                </Text>
                <Text size="sm" c="dimmed" style={{ whiteSpace: "nowrap" }}>
                    {value ?? formatPercentage(ratio)}
                </Text>
            </Group>
            <Progress.Root size="lg">
                {segments.map((segment) => (
                    <Progress.Section
                        key={segment.color}
                        value={total > 0 ? (segment.value * 100) / total : 0}
                        color={segment.color}
                    >
                        <Progress.Label>{segment.label}</Progress.Label>
                    </Progress.Section>
                ))}
            </Progress.Root>
        </div>
    );
}
