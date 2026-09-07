import { Stack } from "@mantine/core";

import type { Snapshot } from "@/types.ts";

import { CgroupPanel } from "./CgroupPanel.tsx";
import { CpuPanel } from "./CpuPanel.tsx";
import { MemoryPanel } from "./MemoryPanel.tsx";
import { NetworkPanel } from "./NetworkPanel.tsx";
import { PressurePanel } from "./PressurePanel.tsx";
import { SystemPanel } from "./SystemPanel.tsx";
import { VolumesPanel } from "./VolumesPanel.tsx";

export function Dashboard({ snapshot }: { snapshot: Snapshot }): React.JSX.Element {
    return (
        <Stack gap="md">
            <SystemPanel snapshot={snapshot} />
            <CpuPanel snapshot={snapshot} />
            <MemoryPanel snapshot={snapshot} />
            <PressurePanel snapshot={snapshot} />
            <CgroupPanel snapshot={snapshot} />
            <NetworkPanel snapshot={snapshot} />
            <VolumesPanel snapshot={snapshot} />
        </Stack>
    );
}
