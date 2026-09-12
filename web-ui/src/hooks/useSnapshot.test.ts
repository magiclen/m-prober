import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";

import { fetchSnapshot } from "@/api.ts";
import { testSnapshot } from "@/test/snapshot.ts";
import type { Snapshot } from "@/types.ts";

import { useSnapshot } from "./useSnapshot.ts";

vi.mock("@/api.ts", () => ({ fetchSnapshot: vi.fn() }));

class Stream extends EventTarget {
    static latest: Stream;
    readonly close = vi.fn();
    constructor() {
        super();
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
