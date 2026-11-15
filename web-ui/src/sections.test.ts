import { expect, test } from "vitest";

import { sectionHref } from "@/sections.ts";

test("links a section to this page rather than to the page base", () => {
    const base = document.createElement("base");
    base.href = "/mprober/";
    document.head.append(base);
    window.history.replaceState(null, "", "/mprober?view=all");
    try {
        const link = document.createElement("a");
        // A bare fragment resolves against the base, which is a different page.
        link.href = "#cpu";
        expect(`${window.location.origin}/mprober/#cpu`).toBe(link.href);
        link.href = sectionHref("cpu");
        expect(`${window.location.origin}/mprober?view=all#cpu`).toBe(link.href);
    } finally {
        base.remove();
        window.history.replaceState(null, "", "/");
    }
});
