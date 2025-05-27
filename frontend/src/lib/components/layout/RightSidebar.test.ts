/**
 * @vitest-environment jsdom
 * @vitest-environment-options { "url": "http://localhost:5173" }
 */
import { describe, it, expect } from "vitest";
import RightSidebar from "./RightSidebar.svelte";

describe("RightSidebar", () => {
  it("should be defined", () => {
    expect(RightSidebar).toBeDefined();
  });

  it("should be a function", () => {
    expect(typeof RightSidebar).toBe("function");
  });

  it("should have correct component structure", () => {
    expect(RightSidebar).toBeDefined();
    expect(typeof RightSidebar).toBe("function");
  });
});
