import { Alert, Button, Card, Center, PasswordInput, Stack, Text, Title } from "@mantine/core";
import { useState } from "react";

import { login } from "@/api.ts";

interface LoginPageProps {
    version: string;
    onAuthenticated: () => void;
}

export function LoginPage({ version, onAuthenticated }: LoginPageProps): React.JSX.Element {
    const [authKey, setAuthKey] = useState("");
    const [rejected, setRejected] = useState(false);
    const [busy, setBusy] = useState(false);

    const submit = (event: React.FormEvent): void => {
        event.preventDefault();

        setBusy(true);
        setRejected(false);

        login(authKey)
            .then((accepted) => {
                if (accepted) {
                    onAuthenticated();
                } else {
                    setRejected(true);
                }
            })
            .catch(() => setRejected(true))
            .finally(() => setBusy(false));
    };

    return (
        <Center mih="100vh" p="md">
            <Card withBorder padding="lg" radius="md" w={360}>
                <form onSubmit={submit}>
                    <Stack gap="md">
                        <div>
                            <Title order={1} size="h3">
                                M Prober
                            </Title>
                            <Text c="dimmed" size="sm">
                                v{version}
                            </Text>
                        </div>

                        <PasswordInput
                            label="Auth key"
                            placeholder="The key this server was started with"
                            value={authKey}
                            onChange={(event) => setAuthKey(event.currentTarget.value)}
                        />

                        {rejected && (
                            <Alert color="red" variant="light">
                                That auth key was rejected.
                            </Alert>
                        )}

                        <Button type="submit" loading={busy} fullWidth>
                            Sign in
                        </Button>
                    </Stack>
                </form>
            </Card>
        </Center>
    );
}
