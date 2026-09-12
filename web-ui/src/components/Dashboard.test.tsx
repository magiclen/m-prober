import { fireEvent, screen, within } from "@testing-library/react";
import { expect, test } from "vitest";

import { Dashboard } from "@/components/Dashboard.tsx";
import { render } from "@/test/render.tsx";
import { testSnapshot } from "@/test/snapshot.ts";

test("renders panels in reading order and exposes copyable system fields", () => {
    render(<Dashboard snapshot={testSnapshot} />);
    expect(
        screen.getAllByRole("heading", { level: 2 }).map((heading) => heading.textContent),
    ).toEqual([
        "Linux Information",
        "Load Average",
        "CPU",
        "Memory",
        "Networks",
        "Volumes",
        "Pressure (PSI)",
        "cgroup",
    ]);
    expect(screen.getByLabelText("Hostname")).toHaveValue("probe-1");
    expect(screen.getByLabelText("Kernel Version")).toHaveValue("6.17.0-40-generic");
    for (const label of ["Hostname", "Kernel Version", "RTC time (UTC)", "Uptime"]) {
        expect(screen.getByLabelText(label)).toHaveAttribute("readonly");
    }
});

test("keeps each CPU group independent while paired readings update", () => {
    const snapshot = {
        ...testSnapshot,
        cpus: [
            testSnapshot.cpus[0],
            { ...testSnapshot.cpus[0], physical_id: 1, model_name: "Second CPU" },
        ],
        cpu_threads: [
            { id: 4, physical_id: 0, usage: 0.3, frequency_mhz: 3600 },
            { id: 1, physical_id: 1, usage: 0.2, frequency_mhz: 800 },
        ],
    };
    const { rerender } = render(<Dashboard snapshot={snapshot} />);
    const first = screen.getByText("Test CPU 2C/2T 3.60GHz").closest("details");
    const second = screen.getByText("Second CPU 2C/2T 800.00MHz").closest("details");
    if (first === null || second === null) {
        throw new Error("Missing CPU groups");
    }
    expect(first).not.toHaveAttribute("open");
    expect(second).not.toHaveAttribute("open");
    fireEvent.click(screen.getByText("Test CPU 2C/2T 3.60GHz"));
    expect(first).toHaveAttribute("open");
    expect(second).not.toHaveAttribute("open");
    expect(within(first).getByRole("progressbar", { name: "CPU4" })).toHaveAttribute(
        "aria-valuenow",
        "30",
    );
    expect(within(first).queryByText("CPU1")).not.toBeInTheDocument();
    expect(within(first).getByText("30.00% (3.60GHz)")).toBeVisible();
    fireEvent.click(screen.getByText("Second CPU 2C/2T 800.00MHz"));
    expect(within(second).getByText("20.00% (800.00MHz)")).toBeVisible();
    rerender(
        <Dashboard
            snapshot={{
                ...snapshot,
                cpu_threads: [{ ...snapshot.cpu_threads[0], usage: 0.4 }, snapshot.cpu_threads[1]],
            }}
        />,
    );
    expect(first).toHaveAttribute("open");
    expect(second).toHaveAttribute("open");
    expect(within(first).getByRole("progressbar", { name: "CPU4" })).toHaveAttribute(
        "aria-valuenow",
        "40",
    );
});

test("keeps missing CPU data unavailable instead of guessing its group or frequency", () => {
    render(
        <Dashboard
            snapshot={{
                ...testSnapshot,
                cpu_threads: [{ id: 8, physical_id: null, usage: null, frequency_mhz: null }],
            }}
        />,
    );
    fireEvent.click(screen.getByText("Unknown CPU"));
    expect(screen.getByText("Unavailable (Unavailable)")).toBeVisible();
    expect(screen.getByRole("progressbar", { name: "CPU8" })).toHaveAttribute("aria-valuenow", "0");
});

test("splits memory into legacy used and cache with matching text and keyboard tooltips", async () => {
    const { rerender } = render(
        <Dashboard
            snapshot={{
                ...testSnapshot,
                memory: {
                    mem: { ...testSnapshot.memory.mem, used: 1024 ** 3 * 2 },
                    swap: {
                        total: 1024 ** 3,
                        used: 1024 ** 2 * 512,
                        free: 1024 ** 2 * 512,
                        cache: 1024 ** 2 * 128,
                    },
                },
            }}
        />,
    );
    const memory = within(screen.getByRole("region", { name: "Memory" }));
    expect(memory.getByRole("progressbar", { name: "Mem Used" })).toHaveAttribute(
        "aria-valuenow",
        "25",
    );
    expect(memory.getByRole("progressbar", { name: "Mem Buffers + cache" })).toHaveAttribute(
        "aria-valuenow",
        "25",
    );
    expect(memory.getByText("1.00 GiB / 4.00 GiB (25.00%)")).toBeInTheDocument();
    expect(memory.getByText("384.00 MiB / 1.00 GiB (37.50%)")).toBeInTheDocument();
    expect(memory.getByRole("progressbar", { name: "Swap Cache" })).toHaveAttribute(
        "aria-valuenow",
        "12.5",
    );
    fireEvent.focus(memory.getByRole("progressbar", { name: "Mem Buffers + cache" }));
    expect(await screen.findByRole("tooltip")).toHaveTextContent(
        "Mem Buffers + cache: 1.00 GiB (25.00%)",
    );
    rerender(
        <Dashboard
            snapshot={{
                ...testSnapshot,
                memory: { ...testSnapshot.memory, swap: { total: 0, used: 0, free: 0, cache: 0 } },
            }}
        />,
    );
    expect(memory.getByRole("progressbar", { name: "Swap Used" })).toHaveAttribute(
        "aria-valuenow",
        "0",
    );
    expect(memory.getByText("Not configured")).toBeInTheDocument();
});

test("groups upload and download rates with their cumulative data", () => {
    render(<Dashboard snapshot={testSnapshot} />);
    const network = within(screen.getByRole("region", { name: "Networks" }));
    expect(network.getAllByRole("columnheader").map((cell) => cell.textContent)).toEqual([
        "Interface",
        "Upload Rate",
        "Uploaded Data",
        "Download Rate",
        "Downloaded Data",
    ]);
    expect(
        within(network.getAllByRole("row")[1])
            .getAllByRole("cell")
            .map((cell) => cell.textContent),
    ).toEqual(["eth0", "500.00 KB/s", "1.00 MB", "1.00 MB/s", "2.00 MB"]);
    const volumes = within(screen.getByRole("region", { name: "Volumes" }));
    expect(volumes.getByText("8.00 MB")).toBeInTheDocument();
    expect(volumes.getByText("4.00 MB")).toBeInTheDocument();
    expect(volumes.getByRole("progressbar", { name: "vda1 usage" })).toHaveAttribute(
        "aria-valuenow",
        "25",
    );
});

test("shows pressure and cgroup summaries before their details", () => {
    render(<Dashboard snapshot={testSnapshot} />);
    const pressure = within(screen.getByRole("region", { name: "Pressure" }));
    expect(pressure.getByRole("progressbar", { name: "CPU" })).toHaveAttribute(
        "aria-valuenow",
        "1.5",
    );
    for (const table of pressure.getAllByRole("table")) {
        expect(table).not.toBeVisible();
    }
    fireEvent.click(pressure.getByText("Pressure details"));
    expect(pressure.getAllByRole("table")).toHaveLength(3);
    expect(pressure.getByText("Unavailable")).toBeVisible();
    const cgroup = within(screen.getByRole("region", { name: "cgroup" }));
    expect(cgroup.getByLabelText("Control group path")).toHaveAttribute("readonly");
    expect(cgroup.getByText("2.00 CPUs")).toBeInTheDocument();
    expect(cgroup.getByText("42 / 4096 (1.03%)")).toBeInTheDocument();
    expect(cgroup.getByText("Total CPU time")).not.toBeVisible();
    fireEvent.click(cgroup.getByText("Control group details"));
    expect(cgroup.getByText("Total CPU time")).toBeVisible();
    expect(cgroup.getByText("3 / 100")).toBeVisible();
});

test("distinguishes missing controllers and unlimited memory", () => {
    const cgroup = testSnapshot.cgroup;
    if (cgroup === null || cgroup.memory === null) {
        throw new Error("The fixture needs a memory controller");
    }
    render(
        <Dashboard
            snapshot={{
                ...testSnapshot,
                cgroup: {
                    ...cgroup,
                    cpu: null,
                    pids: null,
                    memory: { ...cgroup.memory, max: null },
                },
            }}
        />,
    );
    const region = within(screen.getByRole("region", { name: "cgroup" }));
    expect(region.getByText("1.00 GiB — No limit set here")).toBeInTheDocument();
    expect(region.queryByRole("progressbar")).not.toBeInTheDocument();
});

test("tells the reader when PSI and cgroup are unavailable", () => {
    render(<Dashboard snapshot={{ ...testSnapshot, pressure: null, cgroup: null }} />);
    expect(screen.getByText(/does not provide PSI/)).toBeInTheDocument();
    expect(screen.getByText(/does not run mprober under cgroup v2/)).toBeInTheDocument();
});

test("aligns volume usage below rates and keeps extra mount points in the last column", () => {
    render(
        <Dashboard
            snapshot={{
                ...testSnapshot,
                volumes: [{ ...testSnapshot.volumes[0], points: ["/", "/home", "/extra"] }],
            }}
        />,
    );
    const volume = within(screen.getByRole("region", { name: "Volumes" }));
    const rows = volume.getAllByRole("row");
    const capacity = within(rows[2]).getAllByRole("cell");
    expect(capacity[1]).toHaveAttribute("colspan", "2");
    expect(within(capacity[1]).getByRole("progressbar")).toHaveAttribute("aria-valuenow", "25");
    expect(capacity[2]).toHaveAttribute("colspan", "2");
    expect(capacity[2]).toHaveTextContent("25.00 GB / 100.00 GB (25.00%)");
    expect(capacity[3]).toHaveTextContent("/home");
    expect(within(rows[3]).getAllByRole("cell")[1]).toHaveTextContent("/extra");
});
