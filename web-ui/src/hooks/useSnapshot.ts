import { useEffect, useState } from "react";

import { fetchSnapshot } from "@/api.ts";
import type { Snapshot } from "@/types.ts";

type ConnectionState = "connecting" | "live" | "lost";

export interface SnapshotState {
    snapshot: Snapshot | null;
    connection: ConnectionState;
}

/**
 * Follow the server-sent snapshots. The first one is also fetched over plain HTTP, so that the page
 * fills in without waiting for the sampler to finish its current round.
 */
export const useSnapshot = (enabled: boolean): SnapshotState => {
    const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
    const [connection, setConnection] = useState<ConnectionState>("connecting");

    useEffect((): (() => void) | undefined => {
        if (!enabled) {
            return undefined;
        }

        let cancelled = false;

        fetchSnapshot()
            .then((initial): void => {
                // A snapshot which arrived over the stream in the meantime is newer than this one.
                if (!cancelled) {
                    setSnapshot((current): Snapshot => current ?? initial);
                }
            })
            .catch((): void => {
                // The stream reports the connection state on its own, so a failure here needs no handling.
            });

        const source = new EventSource("/api/all/stream", { withCredentials: true });

        source.addEventListener("message", (event: MessageEvent<string>): void => {
            if (cancelled) {
                return;
            }

            const parsed: unknown = JSON.parse(event.data);

            // This page is served by the same binary that produces the event, so the two always agree.
            // oxlint-disable-next-line typescript/no-unsafe-type-assertion
            setSnapshot(parsed as Snapshot);
            setConnection("live");
        });

        // The browser reconnects on its own, so this only has to show that the data is stale.
        source.addEventListener("error", (): void => {
            if (!cancelled) {
                setConnection("lost");
            }
        });

        return (): void => {
            cancelled = true;
            source.close();
        };
    }, [enabled]);

    return { snapshot, connection };
};
