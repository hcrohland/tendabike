import { cleanup } from "@testing-library/svelte";
import { afterEach, vi } from "vitest";
import { setLocale } from "../../paraglide/runtime";

setLocale("en", { reload: false });

Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

if (!HTMLDialogElement.prototype.showModal) {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute("open", "");
  };
}
if (!HTMLDialogElement.prototype.show) {
  HTMLDialogElement.prototype.show = function () {
    this.setAttribute("open", "");
  };
}
if (!HTMLDialogElement.prototype.close) {
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute("open");
  };
}

if (!HTMLElement.prototype.showPopover) {
  HTMLElement.prototype.showPopover = function () {
    this.dispatchEvent(
      Object.assign(new Event("toggle", { bubbles: true }), {
        newState: "open",
      }),
    );
  };
}
if (!HTMLElement.prototype.hidePopover) {
  HTMLElement.prototype.hidePopover = function () {
    this.dispatchEvent(
      Object.assign(new Event("toggle", { bubbles: true }), {
        newState: "closed",
      }),
    );
  };
}

if (!Element.prototype.animate) {
  // Svelte 5 transitions run on the Web Animations API and complete via
  // `animation.onfinish`, so the stub must invoke it (on a timer, so fake
  // timers can drive it) instead of leaving it dangling.
  Element.prototype.animate = function (
    _keyframes?: Keyframe[] | PropertyIndexedKeyframes | null,
    options?: number | KeyframeAnimationOptions,
  ) {
    const duration =
      typeof options === "number"
        ? options
        : typeof options?.duration === "number"
          ? options.duration
          : 0;
    const animation = {
      finished: new Promise((resolve) => {
        setTimeout(resolve, duration);
      }),
      cancel: () => {},
      onfinish: null as null | (() => void),
    };
    setTimeout(() => {
      animation.onfinish?.();
    }, duration);
    return animation as unknown as Animation;
  };
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

// Note: vi.unstubAllGlobals() runs after every test. If a test file uses
// vi.stubGlobal("fetch", ...) in beforeAll, the stub will be removed after the
// first test's cleanup. Always create stubs in beforeEach so they are
// re-established before each test.
