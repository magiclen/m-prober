import { MantineProvider } from "@mantine/core";

import "@mantine/core/styles.css";
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
