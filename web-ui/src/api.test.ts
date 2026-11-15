import { afterEach, expect, test, vi } from "vitest";

import { fetchAuthStatus, fetchConfig, fetchSnapshot, login, logout } from "./api.ts";

afterEach(() => {
    vi.unstubAllGlobals();
});

test.each(["/", "/mprober/", "/admin/tools/mprober/"])(
    "uses the page base %s for HTTP requests",
    async (basePath) => {
        const base = document.createElement("base");
        base.href = basePath;
        document.head.append(base);
        const fetchMock = vi
            .fn<typeof fetch>()
            .mockImplementation(() => Promise.resolve(new Response("{}")));
        vi.stubGlobal("fetch", fetchMock);
        try {
            await fetchConfig();
            await fetchAuthStatus();
            await fetchSnapshot();
            expect(await login("magic")).toBe(true);
            await logout();

            expect(fetchMock.mock.calls.map(([url]) => url)).toEqual(
                ["config", "auth", "all", "auth", "logout"].map(
                    (path) => `${window.location.origin}${basePath}api/${path}`,
                ),
            );
        } finally {
            base.remove();
        }
    },
);
