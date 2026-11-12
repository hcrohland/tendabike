import { fireEvent, render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it } from "vitest";
import ConnectProviders from "./ConnectProviders.svelte";

describe("ConnectProviders", () => {
  /// jsdom does not navigate: the component's `window.location.href = ...`
  /// assignment lands on this stub instead.
  let location: { href: string };

  beforeEach(() => {
    location = { href: "http://localhost/about" };
    Object.defineProperty(window, "location", {
      value: location,
      writable: true,
    });
  });

  it("renders one hero card per supported provider", () => {
    render(ConnectProviders);
    expect(screen.getByText("Strava")).toBeTruthy();
    expect(screen.getByText("intervals.icu")).toBeTruthy();
  });

  it("clicking the Strava card starts the Strava OAuth flow", () => {
    render(ConnectProviders);
    fireEvent.click(screen.getByRole("button", { name: "Connect Strava" }));
    expect(location.href).toBe("/strava/login");
  });

  it("clicking the intervals.icu card starts the icu OAuth flow", () => {
    render(ConnectProviders);
    fireEvent.click(
      screen.getByRole("button", { name: "Connect intervals.icu" }),
    );
    expect(location.href).toBe("/icu/login");
  });

  it("clicking the card body (not only the button) also starts the flow", () => {
    render(ConnectProviders);
    fireEvent.click(screen.getByText("Rides you log in Strava"));
    expect(location.href).toBe("/strava/login");
  });
});
