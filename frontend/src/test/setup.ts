import { afterEach, vi } from "vitest";
import { setLocale } from "../../paraglide/runtime";

setLocale("en", { reload: false });

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});
