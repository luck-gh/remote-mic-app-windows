import { describe, expect, it } from "vitest";
import fixture from "../../contracts/ipc/template-catalog-builtins.json";
import type { TemplateCatalogEntry } from "./bridge";
describe("fixed-key template IPC contract", () => {
  it("exposes one mapping model and leaves UI-dependent actions unconfigured", () => {
    const entries = fixture as TemplateCatalogEntry[];
    expect(entries.map(t => t.id)).toEqual(["preset-agent", "preset-chat", "preset-browser"]);
    for (const entry of entries) {
      expect(entry.kind).toBe("direct"); expect(entry.readOnly).toBe(true);
      expect(entry).not.toHaveProperty("semanticTemplate");
      expect(entry.buttonMappings?.actions.left?.single).toEqual({type:"shortcut",chord:{keys:["left"]}});
      expect(entry.buttonMappings?.actions.home).toBeUndefined();
      expect(entry.buttonMappings?.actions.tv).toBeUndefined();
      expect(entry.buttonMappings?.actions.ok?.long).toEqual({type:"disabled"});
    }
  });
});
