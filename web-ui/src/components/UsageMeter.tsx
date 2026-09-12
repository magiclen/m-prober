import { Progress, Tooltip } from "@mantine/core";
import { Fragment } from "react";

import { formatPercentage } from "@/format.ts";
import { segmentWidths, usageColor } from "@/usage.ts";

export interface UsageSegment {
    label: string;
    value: number;
    color?: string;
    description?: string;
}

interface UsageMeterProps {
    label: string;
    used: number;
    total: number;
    color?: string;
    text?: string;
    segments?: UsageSegment[];
}

export function UsageMeter({
    label,
    used,
    total,
    color,
    text,
    segments,
}: UsageMeterProps): React.JSX.Element {
    const parts = segments ?? [{ label, value: used, color }];
    const widths = segmentWidths(
        parts.map((part) => part.value),
        total,
    );
    return (
        <Progress.Root size="xl">
            {parts.map((segment, index) => {
                const ratio = total > 0 ? segment.value / total : 0;
                const width = widths[index];
                const description =
                    segment.description ?? text ?? (total > 0 ? formatPercentage(ratio) : "N/A");
                const section = (
                    <Progress.Section
                        value={width}
                        color={segment.color ?? usageColor(ratio)}
                        style={{ flexShrink: 0 }}
                        withAria={false}
                        // Mantine draws the bar; custom ARIA text preserves unclipped values.
                        // oxlint-disable-next-line jsx-a11y/prefer-tag-over-role
                        role="progressbar"
                        aria-label={segment.label}
                        aria-valuemin={0}
                        aria-valuemax={100}
                        aria-valuenow={width}
                        aria-valuetext={description}
                        tabIndex={segments === undefined ? undefined : 0}
                    />
                );
                return segments === undefined ? (
                    <Fragment key={segment.label}>{section}</Fragment>
                ) : (
                    <Tooltip
                        key={segment.label}
                        label={`${segment.label}: ${description}`}
                        events={{ hover: true, focus: true, touch: false }}
                        withArrow
                    >
                        {section}
                    </Tooltip>
                );
            })}
        </Progress.Root>
    );
}
