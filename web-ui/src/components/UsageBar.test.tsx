import { screen } from "@testing-library/react";
import { expect, test } from "vitest";

import { render } from "@/test/render.tsx";
import { usageColor } from "@/usage.ts";

import { UsageBar } from "./UsageBar.tsx";

test("uses the original percentage color bands", () => {
    expect([0.49, 0.5, 0.69, 0.7, 0.89, 0.9].map(usageColor)).toEqual([
        "blue",
        "green",
        "green",
        "yellow",
        "yellow",
        "red",
    ]);
});

test("caps a load bar without hiding overload and handles an unknown denominator", () => {
    const { rerender } = render(<UsageBar label="1 minute" used={3} total={2} value="3.00" />);
    expect(screen.getByText("3.00 (150.00%)")).toBeInTheDocument();
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "100");
    rerender(<UsageBar label="1 minute" used={3} total={0} value="3.00" />);
    expect(screen.getByText("3.00 (N/A)")).toBeInTheDocument();
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "0");
});

test("clips stacked sections to the available width without stretching small or zero values", () => {
    const { rerender } = render(
        <UsageBar
            label="Mem"
            used={25}
            total={100}
            segments={[
                { label: "Used", value: 25 },
                { label: "Cache", value: 25 },
            ]}
        />,
    );
    expect(screen.getByRole("progressbar", { name: "Used" })).toHaveAttribute(
        "aria-valuenow",
        "25",
    );
    expect(screen.getByRole("progressbar", { name: "Cache" })).toHaveAttribute(
        "aria-valuenow",
        "25",
    );
    rerender(
        <UsageBar
            label="Mem"
            used={80}
            total={100}
            segments={[
                { label: "Used", value: 80 },
                { label: "Cache", value: 40 },
            ]}
        />,
    );
    expect(screen.getByRole("progressbar", { name: "Cache" })).toHaveAttribute(
        "aria-valuenow",
        "20",
    );
    rerender(
        <UsageBar
            label="Mem"
            used={0}
            total={100}
            segments={[
                { label: "Used", value: 0 },
                { label: "Cache", value: 0 },
            ]}
        />,
    );
    expect(screen.getByRole("progressbar", { name: "Cache" })).toHaveAttribute(
        "aria-valuenow",
        "0",
    );
});
