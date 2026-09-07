import { SimpleGrid, Text } from "@mantine/core";

import { formatDateTime, formatDuration } from "@/format.ts";
import type { Snapshot } from "@/types.ts";

import { Panel } from "./Panel.tsx";

function Field({ name, value }: { name: string; value: string }): React.JSX.Element {
    return (
        <div>
            <Text size="xs" c="dimmed" tt="uppercase">
                {name}
            </Text>
            <Text size="sm" fw={500} style={{ wordBreak: "break-word" }}>
                {value}
            </Text>
        </div>
    );
}

export function SystemPanel({ snapshot }: { snapshot: Snapshot }): React.JSX.Element {
    return (
        <Panel title="System">
            <SimpleGrid cols={{ base: 1, xs: 2, md: 4 }} spacing="sm">
                <Field name="Hostname" value={snapshot.hostname} />
                <Field name="Kernel" value={snapshot.kernel} />
                <Field name="Uptime" value={formatDuration(snapshot.uptime.total_uptime)} />
                <Field name="RTC time (UTC)" value={formatDateTime(snapshot.rtc_time)} />
            </SimpleGrid>
        </Panel>
    );
}
