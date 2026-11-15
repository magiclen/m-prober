import { useEffect, useState } from "react";

import { apiUrl, fetchSnapshot } from "@/api.ts";
import type { Snapshot } from "@/types.ts";

/** How long the page waits before it tries again to reach a probe it lost, in milliseconds. */
export const RETRY_INTERVAL = 3000;

type ConnectionState = "connecting" | "live" | "lost";

export interface SnapshotState {
    snapshot: Snapshot | null;
    connection: ConnectionState;
    lastReceivedAt: number | null;
}

const initialState: SnapshotState = {
    snapshot: null,
    connection: "connecting",
    lastReceivedAt: null,
};

/** Follow streamed snapshots and use HTTP to fill the page before the first stream message. */
export const useSnapshot = (enabled: boolean): SnapshotState => {
    const [state, setState] = useState<SnapshotState>(initialState);

    const [previousEnabled, setPreviousEnabled] = useState(enabled);
    if (previousEnabled !== enabled) {
        setPreviousEnabled(enabled);
        setState(initialState);
    }

    useEffect((): (() => void) | undefined => {
        if (!enabled) {
            return undefined;
        }
        let cancelled = false;
        let receivedStream = false;
        let source: EventSource | undefined;
        let retry: ReturnType<typeof setTimeout> | undefined;

        fetchSnapshot()
            .then((snapshot): void => {
                // A late HTTP response must not replace a streamed snapshot or its receive time.
                if (!cancelled && !receivedStream) {
                    const lastReceivedAt = Date.now();
                    setState((current) => ({ ...current, snapshot, lastReceivedAt }));
                }
            })
            .catch((): void => {
                // The stream reports the connection state on its own.
            });

        const connect = (): void => {
            const stream = new EventSource(apiUrl("all/stream"), { withCredentials: true });
            source = stream;
            stream.addEventListener("message", (event: MessageEvent<string>): void => {
                if (cancelled) {
                    return;
                }
                const parsed: unknown = JSON.parse(event.data);
                receivedStream = true;
                // This page and the API are served by the same binary.
                // oxlint-disable-next-line typescript/no-unsafe-type-assertion
                const snapshot = parsed as Snapshot;
                setState({ snapshot, connection: "live", lastReceivedAt: Date.now() });
            });
            stream.addEventListener("error", (): void => {
                if (cancelled) {
                    return;
                }
                setState((current) => ({ ...current, connection: "lost" }));
                // A browser reconnects by itself after a network error, but gives up after an HTTP error such as a 502 from a proxy.
                if (stream.readyState === EventSource.CLOSED) {
                    retry = setTimeout(connect, RETRY_INTERVAL);
                }
            });
        };
        connect();

        return (): void => {
            cancelled = true;
            clearTimeout(retry);
            source?.close();
        };
    }, [enabled]);

    return enabled ? state : initialState;
};
