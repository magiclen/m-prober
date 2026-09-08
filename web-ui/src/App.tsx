import {
    ActionIcon,
    Badge,
    Container,
    Group,
    Loader,
    Stack,
    Text,
    Title,
    Tooltip,
    useMantineColorScheme,
} from "@mantine/core";
import { useCallback, useEffect, useState } from "react";

import { fetchAuthStatus, fetchConfig, logout } from "@/api.ts";
import { Dashboard } from "@/components/Dashboard.tsx";
import { LoginPage } from "@/components/LoginPage.tsx";
import { useSnapshot } from "@/hooks/useSnapshot.ts";
import type { AuthStatus, Config } from "@/types.ts";

function ColorSchemeToggle(): React.JSX.Element {
    const { colorScheme, toggleColorScheme } = useMantineColorScheme();

    return (
        <Tooltip label="Toggle the color scheme">
            <ActionIcon
                variant="default"
                size="lg"
                onClick={toggleColorScheme}
                aria-label="Toggle the color scheme"
            >
                {colorScheme === "dark" ? "☀" : "☾"}
            </ActionIcon>
        </Tooltip>
    );
}

export function App(): React.JSX.Element {
    const [config, setConfig] = useState<Config | null>(null);
    const [auth, setAuth] = useState<AuthStatus | null>(null);

    const refreshAuth = useCallback(() => {
        fetchAuthStatus()
            .then(setAuth)
            .catch(() => setAuth(null));
    }, []);

    useEffect(() => {
        fetchConfig()
            .then(setConfig)
            .catch(() => setConfig(null));
        refreshAuth();
    }, [refreshAuth]);

    const authenticated = auth?.authenticated ?? false;
    const { snapshot, connection } = useSnapshot(authenticated);

    // A browser retries a dropped stream on its own and never gives up, so an expired session would
    // otherwise leave the page saying "reconnecting" for good instead of asking to sign in again.
    useEffect(() => {
        if (connection === "lost") {
            refreshAuth();
        }
    }, [connection, refreshAuth]);

    if (config === null || auth === null) {
        return (
            <Group justify="center" mih="100vh">
                <Loader />
            </Group>
        );
    }

    if (!authenticated) {
        return <LoginPage version={config.version} onAuthenticated={refreshAuth} />;
    }

    return (
        <Container fluid py="md">
            <Stack gap="md">
                <Group justify="space-between" wrap="nowrap">
                    <Group gap="sm" align="baseline">
                        <Title order={1} size="h3">
                            M Prober
                        </Title>
                        <Text c="dimmed" size="sm">
                            v{config.version}
                        </Text>
                    </Group>

                    <Group gap="sm">
                        <Badge color={connection === "live" ? "teal" : "orange"} variant="light">
                            {connection === "live"
                                ? `every ${config.detect_interval}s`
                                : connection === "connecting"
                                  ? "connecting"
                                  : "reconnecting"}
                        </Badge>

                        {auth.required && (
                            <Tooltip label="Sign out">
                                <ActionIcon
                                    variant="default"
                                    size="lg"
                                    aria-label="Sign out"
                                    onClick={() => {
                                        void logout().finally(refreshAuth);
                                    }}
                                >
                                    ⏻
                                </ActionIcon>
                            </Tooltip>
                        )}

                        <ColorSchemeToggle />
                    </Group>
                </Group>

                {snapshot === null ? (
                    <Group justify="center" py="xl">
                        <Loader />
                    </Group>
                ) : (
                    <Dashboard snapshot={snapshot} />
                )}
            </Stack>
        </Container>
    );
}
