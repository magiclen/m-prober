import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";

const getComputedStyle = window.getComputedStyle.bind(window);

window.getComputedStyle = (element: Element): CSSStyleDeclaration => getComputedStyle(element);
window.HTMLElement.prototype.scrollIntoView = vi.fn<HTMLElement["scrollIntoView"]>();

Object.defineProperty(window, "matchMedia", {
    configurable: true,
    writable: true,
    value: vi.fn().mockImplementation((query: string): MediaQueryList => ({
        matches: false,
        media: query,
        onchange: null,
        addListener: vi.fn(),
        removeListener: vi.fn(),
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        dispatchEvent: vi.fn(() => false),
    })),
});

if (!("fonts" in document)) {
    Object.defineProperty(document, "fonts", {
        configurable: true,
        writable: true,
        value: {
            addEventListener: vi.fn(),
            removeEventListener: vi.fn(),
        } satisfies Pick<FontFaceSet, "addEventListener" | "removeEventListener">,
    });
}

class ResizeObserverMock implements ResizeObserver {
    readonly disconnect = vi.fn<ResizeObserver["disconnect"]>();
    readonly observe = vi.fn<ResizeObserver["observe"]>();
    readonly unobserve = vi.fn<ResizeObserver["unobserve"]>();
}

window.ResizeObserver = ResizeObserverMock;

afterEach(cleanup);
