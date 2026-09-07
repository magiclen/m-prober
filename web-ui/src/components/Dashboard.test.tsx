import { screen } from "@testing-library/react";
import { expect, test } from "vitest";

import { Dashboard } from "@/components/Dashboard.tsx";
import { render } from "@/test/render.tsx";
import { testSnapshot } from "@/test/snapshot.ts";

test("renders every panel", () => {
    render(<Dashboard snapshot={testSnapshot} />);

    for (const title of [
        "System",
        "CPU",
        "Memory",
        "Pressure (PSI)",
        "cgroup",
        "Network",
        "Volumes",
    ]) {
        expect(screen.getByRole("heading", { name: title })).toBeInTheDocument();
    }
});

test("shows the values a reader looks for first", () => {
    render(<Dashboard snapshot={testSnapshot} />);

    expect(screen.getByText("probe-1")).toBeInTheDocument();
    expect(screen.getByText("6.17.0-40-generic")).toBeInTheDocument();
    expect(screen.getByText("2d 3h 4m 5s")).toBeInTheDocument();

    // The first entry of `cpus_stat` is the average over every CPU.
    expect(screen.getByText("25.00%")).toBeInTheDocument();

    expect(screen.getByText("1.00 GiB / 4.00 GiB")).toBeInTheDocument();
    expect(screen.getByText("1.00 MB/s")).toBeInTheDocument();
    expect(screen.getByText("42 / 4096")).toBeInTheDocument();
});

test("tells the reader when the kernel provides no PSI", () => {
    render(<Dashboard snapshot={{ ...testSnapshot, pressure: null }} />);

    expect(screen.getByText(/does not provide PSI/)).toBeInTheDocument();
});

test("tells the reader when there is no cgroup", () => {
    render(<Dashboard snapshot={{ ...testSnapshot, cgroup: null }} />);

    expect(screen.getByText(/does not run mprober under cgroup v2/)).toBeInTheDocument();
});
