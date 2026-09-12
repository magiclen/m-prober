import {
    ActionIcon,
    Badge,
    Burger,
    NavLink,
    Group,
    Loader,
    Stack,
    Text,
    Title,
    Tooltip,
    useMantineColorScheme,
} from "@mantine/core";
import { useCallback, useEffect, useRef, useState } from "react";

import { fetchAuthStatus, fetchConfig, logout } from "@/api.ts";
import { Dashboard } from "@/components/Dashboard.tsx";
import { LastUpdate } from "@/components/LastUpdate.tsx";
import { LoginPage } from "@/components/LoginPage.tsx";
import { useSnapshot } from "@/hooks/useSnapshot.ts";
import { sections } from "@/sections.ts";
import type { AuthStatus, Config } from "@/types.ts";

import classes from "./App.module.css";

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
    const [menuOpened, setMenuOpened] = useState(false);
    const [activeSection, setActiveSection] = useState<string>(sections[0].id);
    const header = useRef<HTMLElement>(null);
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
    const { snapshot, connection, lastReceivedAt } = useSnapshot(authenticated);

    // A browser retries a dropped stream on its own and never gives up, so an expired session would
    // otherwise leave the page saying "reconnecting" for good instead of asking to sign in again.
    useEffect(() => {
        if (connection === "lost") {
            refreshAuth();
        }
    }, [connection, refreshAuth]);

    const hasSnapshot = snapshot !== null;
    useEffect((): (() => void) | undefined => {
        if (!authenticated || !hasSnapshot) {
            return undefined;
        }
        const updateSection = (): void => {
            const edge = (header.current?.getBoundingClientRect().bottom ?? 0) + 32;
            let selected: string = sections[0].id;
            for (const section of sections) {
                const element = document.getElementById(section.id);
                if (element !== null && element.getBoundingClientRect().top <= edge) {
                    selected = section.id;
                }
            }
            if (
                window.scrollY > 0 &&
                window.innerHeight + window.scrollY >= document.documentElement.scrollHeight - 2
            ) {
                selected = sections[sections.length - 1].id;
            }
            setActiveSection(selected);
        };
        updateSection();
        window.addEventListener("scroll", updateSection, { passive: true });
        window.addEventListener("resize", updateSection);
        return (): void => {
            window.removeEventListener("scroll", updateSection);
            window.removeEventListener("resize", updateSection);
        };
    }, [authenticated, hasSnapshot]);

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
        <div className={classes.shell}>
            <header ref={header} className={classes.header}>
                <Stack gap="xs">
                    <Group justify="space-between" gap="xs" className={classes.toolbar}>
                        <Group gap="sm">
                            <Burger
                                className={classes.burger}
                                size="sm"
                                opened={menuOpened}
                                onClick={() => setMenuOpened((opened) => !opened)}
                                aria-label="Toggle navigation"
                                aria-expanded={menuOpened}
                                aria-controls="dashboard-navigation"
                            />
                            <Title order={1} size="h3">
                                M Prober
                            </Title>
                            <Text c="dimmed" size="sm">
                                v{config.version}
                            </Text>
                        </Group>

                        <Group gap="sm">
                            <Badge
                                color={connection === "live" ? "teal" : "orange"}
                                variant="light"
                            >
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

                    <LastUpdate lastReceivedAt={lastReceivedAt} />
                </Stack>
            </header>
            {menuOpened && (
                <button
                    className={classes.backdrop}
                    aria-label="Close navigation"
                    onClick={() => setMenuOpened(false)}
                />
            )}
            <nav
                id="dashboard-navigation"
                aria-label="Information categories"
                className={classes.navigation}
                data-open={menuOpened || undefined}
            >
                {sections.map((section) => (
                    <NavLink
                        key={section.id}
                        onKeyDown={(event) => {
                            if (event.key === "Escape") {
                                setMenuOpened(false);
                                header.current?.querySelector("button")?.focus();
                            }
                        }}
                        component="a"
                        href={`#${section.id}`}
                        label={section.label}
                        active={activeSection === section.id}
                        aria-current={activeSection === section.id ? "location" : undefined}
                        onClick={() => {
                            setActiveSection(section.id);
                            setMenuOpened(false);
                            document.getElementById(section.id)?.focus({ preventScroll: true });
                        }}
                    />
                ))}
            </nav>
            <main className={classes.main}>
                {snapshot === null ? (
                    <Group justify="center" py="xl">
                        <Loader />
                    </Group>
                ) : (
                    <Dashboard snapshot={snapshot} />
                )}
            </main>
        </div>
    );
}
