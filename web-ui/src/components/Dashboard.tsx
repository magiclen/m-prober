import { Stack } from "@mantine/core";

import type { Snapshot } from "@/types.ts";

import { CgroupPanel } from "./CgroupPanel.tsx";
import { CpuPanel } from "./CpuPanel.tsx";
import { MemoryPanel } from "./MemoryPanel.tsx";
import { NetworkPanel } from "./NetworkPanel.tsx";
import { PressurePanel } from "./PressurePanel.tsx";
import { SystemPanel } from "./SystemPanel.tsx";
import { VolumesPanel } from "./VolumesPanel.tsx";

import classes from "./Dashboard.module.css";

export function Dashboard({ snapshot }: { snapshot: Snapshot }): React.JSX.Element {
    return (
        <Stack gap="md">
            {/* These two read better across the whole width, so they stay out of the grid: an item
                spanning every column would keep the grid from collapsing its empty columns. */}
            <SystemPanel snapshot={snapshot} />
            <CpuPanel snapshot={snapshot} />

            <div className={classes.grid}>
                <MemoryPanel snapshot={snapshot} />
                <PressurePanel snapshot={snapshot} />
                <CgroupPanel snapshot={snapshot} />
                <NetworkPanel snapshot={snapshot} />
                <VolumesPanel snapshot={snapshot} />
            </div>
        </Stack>
    );
}
