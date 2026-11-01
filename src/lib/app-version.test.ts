import { describe, expect, it } from "vitest";
import { displayAppVersion } from "./app-version";

describe("personal version display", () => {
  it.each([
    ["1.0.0+1", "1.0.0.1"],
    ["1.0.0+10", "1.0.0.10"],
    ["1.0.1+11", "1.0.1.11"],
    ["1.0.0", "1.0.0"],
    ["1.0.0-rc.1", "1.0.0-rc.1"],
    ["1.0.0+source.hash", "1.0.0+source.hash"],
    ["—", "—"],
  ])("%s is displayed as %s", (canonical, display) => {
    expect(displayAppVersion(canonical)).toBe(display);
  });
});
