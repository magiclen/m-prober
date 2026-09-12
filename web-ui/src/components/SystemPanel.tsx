import { SimpleGrid, TextInput } from "@mantine/core";

import { formatDateTime, formatDuration } from "@/format.ts";
import type { Snapshot } from "@/types.ts";

import { Panel } from "./Panel.tsx";

export function SystemPanel({ snapshot }: { snapshot: Snapshot }): React.JSX.Element {
    return (
        <Panel title="Linux Information">
            <SimpleGrid cols={{ base: 1, sm: 2 }}>
                <TextInput label="Kernel Version" value={snapshot.kernel} readOnly />
                <TextInput label="Hostname" value={snapshot.hostname} readOnly />
                <TextInput
                    label="RTC time (UTC)"
                    value={formatDateTime(snapshot.rtc_time)}
                    readOnly
                />
                <TextInput
                    label="Uptime"
                    value={formatDuration(snapshot.uptime.total_uptime)}
                    readOnly
                />
            </SimpleGrid>
        </Panel>
    );
}
