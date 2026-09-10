import { describe, expect, it } from "vitest";
import { navigationItems } from "./navigation";

describe("Windows navigation", () => {
  it("puts the driver guide before settings pages without empty entries", () => {
    expect(navigationItems.map((item) => item.id)).toEqual([
      "drivers",
      "buttons",
      "connection",
      "permissions",
      "about",
    ]);
    expect(navigationItems.every((item) => item.label.length > 0)).toBe(true);
  });
});
