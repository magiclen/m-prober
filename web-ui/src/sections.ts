export const sections = [
    { id: "linux-information", label: "Linux Information" },
    { id: "load-average", label: "Load Average" },
    { id: "cpu", label: "CPU" },
    { id: "memory", label: "Memory" },
    { id: "networks", label: "Networks" },
    { id: "volumes", label: "Volumes" },
    { id: "pressure", label: "Pressure" },
    { id: "cgroup", label: "cgroup" },
] as const;

/**
 * Link a section to this page itself, since a bare `#id` resolves against `<base>` and would reload
 * a page opened at another path such as `/mprober`.
 */
export const sectionHref = (id: string): string =>
    `${window.location.pathname}${window.location.search}#${id}`;
