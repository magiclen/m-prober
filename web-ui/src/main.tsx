import { MantineProvider } from "@mantine/core";

import "@mantine/core/styles.css";
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
