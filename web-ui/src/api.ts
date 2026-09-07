import type { AuthStatus, Config, Snapshot } from "@/types.ts";

const getJson = async <T>(path: string): Promise<T> => {
    const response = await fetch(path, { credentials: "same-origin" });

    if (!response.ok) {
        throw new Error(`${path} responded with ${response.status}`);
    }

    const body: unknown = await response.json();

    // This page is served by the same binary that produces the body, so the two always agree.
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion
    return body as T;
};

export const fetchConfig = (): Promise<Config> => getJson<Config>("/api/config");

export const fetchAuthStatus = (): Promise<AuthStatus> => getJson<AuthStatus>("/api/auth");

export const fetchSnapshot = (): Promise<Snapshot> => getJson<Snapshot>("/api/all");

/**
 * Resolves to whether the key was accepted, since a wrong key is an expected answer rather than a
 * failure.
 */
export const login = async (authKey: string): Promise<boolean> => {
    const response = await fetch("/api/auth", {
        method: "POST",
        credentials: "same-origin",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ auth_key: authKey }),
    });

    return response.ok;
};

export const logout = async (): Promise<void> => {
    await fetch("/api/logout", { method: "POST", credentials: "same-origin" });
};
