import { Card, Group, Text, Title } from "@mantine/core";
import type { ReactNode } from "react";

interface PanelProps {
    title: string;
    children: ReactNode;
}

export function Panel({ title, children }: PanelProps): React.JSX.Element {
    return (
        <Card withBorder padding="md" radius="md">
            <Group justify="space-between" align="baseline" mb="sm" wrap="nowrap">
                {/* The name of a panel is short, so it never needs to wrap. */}
                <Title order={2} size="h5" style={{ whiteSpace: "nowrap" }}>
                    {title}
                </Title>
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
