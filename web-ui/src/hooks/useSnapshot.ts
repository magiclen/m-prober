import { useEffect, useState } from "react";

import { fetchSnapshot } from "@/api.ts";
import type { Snapshot } from "@/types.ts";

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

        const source = new EventSource("/api/all/stream", { withCredentials: true });
        source.addEventListener("message", (event: MessageEvent<string>): void => {
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
        source.addEventListener("error", (): void => {
            if (!cancelled) {
                setState((current) => ({ ...current, connection: "lost" }));
            }
        });
        return (): void => {
            cancelled = true;
            source.close();
        };
    }, [enabled]);

    return enabled ? state : initialState;
};
