import { MantineProvider } from "@mantine/core";

import "@mantine/core/styles.css";
// Self-hosted, so that a probe with no route to the internet still renders with its own font. The
// variable build covers every weight in one file per subset.
import "@fontsource-variable/roboto-mono/index.css";
// Overrides `--mantine-scale`, so it has to come after the styles which define it.
import "@/scale.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "@/App.tsx";
import { theme } from "@/theme.ts";

const appElement = document.querySelector<HTMLDivElement>("#app");

if (!appElement) {
    throw new Error("App element was not found");
}

createRoot(appElement).render(
    <StrictMode>
        <MantineProvider theme={theme} defaultColorScheme="auto">
            <App />
        </MantineProvider>
    </StrictMode>,
);
