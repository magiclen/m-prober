import { act, screen } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";

import { render } from "@/test/render.tsx";

import { LastUpdate } from "./LastUpdate.tsx";

afterEach(() => vi.useRealTimers());

test("shows Never before receiving data and advances elapsed time without new data", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-09-12T00:00:00Z"));
    const { rerender } = render(<LastUpdate lastReceivedAt={null} />);
    expect(screen.getByText("Last Update Time: Never")).toBeInTheDocument();
    const receivedAt = Date.now();
    rerender(<LastUpdate lastReceivedAt={receivedAt} />);
    expect(screen.getByText(new Date(receivedAt).toLocaleString())).toHaveAttribute(
        "datetime",
        "2026-09-12T00:00:00.000Z",
    );
    await act(() => vi.advanceTimersByTime(12_000));
    expect(screen.getByText(/12s ago/)).toBeInTheDocument();
});
