import { Alert, Button, Card, Center, PasswordInput, Stack, Text, Title } from "@mantine/core";
import { useState } from "react";

import { login } from "@/api.ts";

interface LoginPageProps {
    version: string;
    onAuthenticated: () => void;
}

export function LoginPage({ version, onAuthenticated }: LoginPageProps): React.JSX.Element {
    const [authKey, setAuthKey] = useState("");
    const [failure, setFailure] = useState<"rejected" | "unreachable" | null>(null);
    const [busy, setBusy] = useState(false);

    const submit = (event: React.FormEvent): void => {
        event.preventDefault();

        setBusy(true);
        setFailure(null);

        login(authKey)
            .then((accepted) => {
                if (accepted) {
                    onAuthenticated();
                } else {
                    setFailure("rejected");
                }
            })
            .catch(() => setFailure("unreachable"))
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

                        {failure !== null && (
                            <Alert color="red" variant="light">
                                {failure === "rejected"
                                    ? "That auth key was rejected."
                                    : "The server could not be reached. Please try again."}
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
