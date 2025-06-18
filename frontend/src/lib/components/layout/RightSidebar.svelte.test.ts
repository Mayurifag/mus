import { describe, it, expect } from "vitest";
import { render } from "@testing-library/svelte";
import RightSidebar from "./RightSidebar.svelte";

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
