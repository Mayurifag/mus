import { describe, it, expect, vi } from "vitest";
import { render } from "@testing-library/svelte";
import RightSidebar from "./RightSidebar.svelte";

// Mock the API client
vi.mock("$lib/services/apiClient", () => ({
  fetchErroredTracks: vi.fn().mockResolvedValue([]),
}));

describe("RightSidebar", () => {
  it("should be defined", () => {
    expect(RightSidebar).toBeDefined();
  });

  it("should render as a div element", () => {
    const { container } = render(RightSidebar);

    expect(container.querySelector("div")).toBeInTheDocument();
  });

  it("should have correct styling classes", () => {
    const { container } = render(RightSidebar);

    const div = container.querySelector("div");
    expect(div).toHaveClass("h-full", "w-full");
  });
});
