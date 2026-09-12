import { Text } from "@mantine/core";
import { useEffect, useState } from "react";

export function LastUpdate({
    lastReceivedAt,
}: {
    lastReceivedAt: number | null;
}): React.JSX.Element {
    const [now, setNow] = useState(Date.now);
    useEffect(() => {
        const timer = window.setInterval(() => setNow(Date.now()), 1000);
        return (): void => window.clearInterval(timer);
    }, []);

    return (
        <Text size="xs" c="dimmed">
            Last Update Time:{" "}
            {lastReceivedAt === null ? (
                "Never"
            ) : (
                <>
                    <time dateTime={new Date(lastReceivedAt).toISOString()}>
                        {new Date(lastReceivedAt).toLocaleString()}
                    </time>
                    {` (${Math.max(0, Math.floor((now - lastReceivedAt) / 1000))}s ago)`}
                </>
            )}
        </Text>
    );
}
