import { Box, Text } from "@mantine/core";
import type { ReactNode } from "react";

import classes from "./Details.module.css";

export function Details({
    label,
    children,
}: {
    label: string;
    children: ReactNode;
}): React.JSX.Element {
    return (
        <details className={classes.details}>
            <summary>
                <Text component="span" size="sm" fw={500}>
                    {label}
                </Text>
            </summary>
            <Box p="md">{children}</Box>
        </details>
    );
}
