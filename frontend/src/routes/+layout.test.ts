import { describe, expect, it, vi, beforeEach } from "vitest";
import { load } from "./+layout";
import { getUser } from "../lib/user.svelte";

vi.mock("../lib/types.svelte", () => ({
  getTypes: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("../lib/user.svelte", () => ({
  initData: vi.fn().mockResolvedValue(undefined),
  getUser: vi.fn(),
}));

function asAnonymous() {
  vi.mocked(getUser).mockReturnValue(undefined);
}

/// The layout only reads `url.pathname`, so a partial event is enough.
function visit(pathname: string) {
  return load({ url: new URL(pathname, "http://localhost") } as any);
}

describe("layout anonymous redirect", () => {
  beforeEach(() => {
    vi.mocked(getUser).mockReset();
  });

  it("lets an anonymous visitor stay on /about (the guide entrance)", async () => {
    asAnonymous();
    await expect(visit("/about")).resolves.toBeUndefined();
  });

  it("lets an anonymous visitor read a guide topic (/about/[topic])", async () => {
    asAnonymous();
    await expect(visit("/about/onboarding")).resolves.toBeUndefined();
  });

  it("lets an anonymous visitor stay on /data-protection", async () => {
    asAnonymous();
    await expect(visit("/data-protection")).resolves.toBeUndefined();
  });

  it("redirects an anonymous visitor elsewhere to /about", async () => {
    asAnonymous();
    await expect(visit("/cat")).rejects.toMatchObject({
      status: 302,
      location: "/about",
    });
  });

  it("does not redirect a signed-in visitor", async () => {
    vi.mocked(getUser).mockReturnValue({} as any);
    await expect(visit("/cat")).resolves.toBeUndefined();
  });
});
