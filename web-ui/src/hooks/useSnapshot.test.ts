import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";

import { fetchSnapshot } from "@/api.ts";
import type * as Api from "@/api.ts";
import { testSnapshot } from "@/test/snapshot.ts";
import type { Snapshot } from "@/types.ts";

import { RETRY_INTERVAL, useSnapshot } from "./useSnapshot.ts";

vi.mock("@/api.ts", async () => ({
    ...(await vi.importActual<typeof Api>("@/api.ts")),
    fetchSnapshot: vi.fn(),
}));

class Stream extends EventTarget {
    static readonly CLOSED = 2;
    static latest: Stream;
    readonly close = vi.fn();
    readonly url: string;
    readyState = 0;
    constructor(url: string) {
        super();
        this.url = url;
        Stream.latest = this;
    }
}

beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-09-12T00:00:00Z"));
    vi.stubGlobal("EventSource", Stream);
});

afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
    vi.resetAllMocks();
});

test("uses the page base for streamed snapshots", async () => {
    const base = document.createElement("base");
    base.href = "/admin/tools/mprober/";
    document.head.append(base);
    vi.mocked(fetchSnapshot).mockResolvedValue(testSnapshot);
    try {
        const { unmount } = renderHook(() => useSnapshot(true));
        await act(async () => {
            await Promise.resolve();
        });
        expect(Stream.latest.url).toBe(
            `${window.location.origin}/admin/tools/mprober/api/all/stream`,
        );
        unmount();
    } finally {
        base.remove();
    }
});

test("dates the accepted HTTP snapshot and each stream update, preserving time on disconnect", async () => {
    vi.mocked(fetchSnapshot).mockResolvedValue(testSnapshot);
    const { result, unmount } = renderHook(() => useSnapshot(true));
    await act(async () => {
        await Promise.resolve();
    });
    const firstTime = Date.now();
    expect(result.current.lastReceivedAt).toBe(firstTime);
    expect(result.current.snapshot).toBe(testSnapshot);
    act(() => {
        vi.advanceTimersByTime(5000);
        Stream.latest.dispatchEvent(
            new MessageEvent("message", {
                data: JSON.stringify({ ...testSnapshot, hostname: "updated" }),
            }),
        );
    });
    expect(result.current.lastReceivedAt).toBe(firstTime + 5000);
    expect(result.current.snapshot?.hostname).toBe("updated");
    act(() => {
        vi.advanceTimersByTime(5000);
        Stream.latest.dispatchEvent(new Event("error"));
    });
    expect(result.current.connection).toBe("lost");
    expect(result.current.lastReceivedAt).toBe(firstTime + 5000);
    unmount();
    expect(Stream.latest.close).toHaveBeenCalledOnce();
});

test("ignores a late HTTP response and resets readings between authenticated sessions", async () => {
    let resolveSnapshot: (value: Snapshot) => void = vi.fn();
    const pending = new Promise<Snapshot>((resolve) => {
        resolveSnapshot = resolve;
    });
    vi.mocked(fetchSnapshot).mockReturnValue(pending);
    const { result, rerender } = renderHook(({ enabled }) => useSnapshot(enabled), {
        initialProps: { enabled: true },
    });
    await act(() =>
        Stream.latest.dispatchEvent(
            new MessageEvent("message", {
                data: JSON.stringify({ ...testSnapshot, hostname: "stream" }),
            }),
        ),
    );
    const receivedAt = result.current.lastReceivedAt;
    await act(async () => {
        vi.advanceTimersByTime(5000);
        resolveSnapshot(testSnapshot);
        await pending;
    });
    expect(result.current.snapshot?.hostname).toBe("stream");
    expect(result.current.lastReceivedAt).toBe(receivedAt);
    rerender({ enabled: false });
    expect(result.current.snapshot).toBeNull();
    expect(result.current.lastReceivedAt).toBeNull();
    vi.mocked(fetchSnapshot).mockReturnValue(new Promise(() => {}));
    rerender({ enabled: true });
    expect(result.current.snapshot).toBeNull();
    expect(result.current.connection).toBe("connecting");
});

test("connects again after an HTTP error closes the stream", () => {
    vi.mocked(fetchSnapshot).mockReturnValue(new Promise(() => {}));
    const { result, unmount } = renderHook(() => useSnapshot(true));
    const closed = Stream.latest;
    act(() => {
        // A 502 from a proxy, for example, makes the browser give the stream up.
        closed.readyState = Stream.CLOSED;
        closed.dispatchEvent(new Event("error"));
    });
    expect("lost").toBe(result.current.connection);
    act(() => {
        vi.advanceTimersByTime(RETRY_INTERVAL);
    });
    expect(Stream.latest).not.toBe(closed);
    act(() => {
        Stream.latest.dispatchEvent(
            new MessageEvent("message", { data: JSON.stringify(testSnapshot) }),
        );
    });
    expect("live").toBe(result.current.connection);
    unmount();
    expect(Stream.latest.close).toHaveBeenCalledOnce();
});
