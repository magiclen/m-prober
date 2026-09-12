import { Text } from "@mantine/core";

import { formatPercentage } from "@/format.ts";

import { UsageMeter } from "./UsageMeter.tsx";
import type { UsageSegment } from "./UsageMeter.tsx";

import classes from "./UsageBar.module.css";

interface UsageBarProps {
    label: string;
    used: number;
    total: number;
    value?: string;
    color?: string;
    text?: string;
    segments?: UsageSegment[];
}

export function UsageBar({
    label,
    used,
    total,
    value,
    color,
    text,
    segments,
}: UsageBarProps): React.JSX.Element {
    const percentage = total > 0 ? formatPercentage(used / total) : "N/A";
    const displayed = text ?? (value === undefined ? percentage : `${value} (${percentage})`);
    return (
        <div className={classes.row}>
            <Text size="sm" fw={500}>
                {label}
            </Text>
            <UsageMeter
                label={label}
                used={used}
                total={total}
                color={color}
                text={displayed}
                segments={segments}
            />
            <Text size="sm" className={classes.value}>
                {displayed}
            </Text>
        </div>
    );
}
