// oxlint-disable typescript/no-unsafe-call typescript/no-unsafe-member-access

import { execFileSync } from "node:child_process";

try {
    execFileSync("git", ["rev-parse", "--git-dir"], {
        stdio: "ignore",
    });
} catch {
    // Not a Git working tree.
    process.exit(0);
}

try {
    execFileSync("git", ["config", "--local", "core.hooksPath", ".githooks"], {
        stdio: "inherit",
    });
} catch (error) {
    console.warn("Failed to set Git hooks path:", error);

    process.exit(0);
}
