import { Stack } from "@mantine/core";

import { sections } from "@/sections.ts";
import type { Snapshot } from "@/types.ts";

import { CgroupPanel } from "./CgroupPanel.tsx";
import { CpuPanel } from "./CpuPanel.tsx";
import { LoadPanel } from "./LoadPanel.tsx";
import { MemoryPanel } from "./MemoryPanel.tsx";
import { NetworkPanel } from "./NetworkPanel.tsx";
import { PressurePanel } from "./PressurePanel.tsx";
import { SystemPanel } from "./SystemPanel.tsx";
import { VolumesPanel } from "./VolumesPanel.tsx";

import classes from "./Dashboard.module.css";

const panels = [
    SystemPanel,
    LoadPanel,
    CpuPanel,
    MemoryPanel,
    NetworkPanel,
    VolumesPanel,
    PressurePanel,
    CgroupPanel,
];

export function Dashboard({ snapshot }: { snapshot: Snapshot }): React.JSX.Element {
    return (
        <Stack gap="md">
            {sections.map((section, index) => {
                const Component = panels[index];
                return (
                    <section
                        key={section.id}
                        id={section.id}
                        aria-label={section.label}
                        tabIndex={-1}
                        className={classes.section}
                    >
                        <Component snapshot={snapshot} />
                    </section>
                );
            })}
        </Stack>
    );
}
