import { MantineProvider } from "@mantine/core";
import { render as testingLibraryRender } from "@testing-library/react";
import type { RenderResult } from "@testing-library/react";
import type { ReactNode } from "react";

import { theme } from "@/theme.ts";

export function render(ui: ReactNode): RenderResult {
    return testingLibraryRender(ui, {
        wrapper: ({ children }) => (
            <MantineProvider env="test" theme={theme}>
                {children}
            </MantineProvider>
        ),
    });
}
