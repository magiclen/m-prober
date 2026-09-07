import { Card, Group, Text, Title } from "@mantine/core";
import type { ReactNode } from "react";

interface PanelProps {
    title: string;
    /** Shown next to the title, for a value which describes the whole panel. */
    aside?: ReactNode;
    children: ReactNode;
}

export function Panel({ title, aside, children }: PanelProps): React.JSX.Element {
    return (
        <Card withBorder padding="md" radius="md">
            <Group justify="space-between" align="baseline" mb="sm" wrap="nowrap">
                <Title order={2} size="h5">
                    {title}
                </Title>
                {aside !== undefined && (
                    <Text c="dimmed" size="sm" style={{ textAlign: "right" }}>
                        {aside}
                    </Text>
                )}
            </Group>
            {children}
        </Card>
    );
}

/** Shown instead of a panel body when the kernel or the environment does not provide the data. */
export function Unsupported({ children }: { children: ReactNode }): React.JSX.Element {
    return (
        <Text c="dimmed" size="sm">
            {children}
        </Text>
    );
}
