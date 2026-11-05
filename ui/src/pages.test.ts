import { describe, expect, it } from "vitest";
import { menu, ordered, type Row } from "./pages";

const rows: Row[] = [
  { id: "a", title: "About", status: "Published", published_at: "2026-11-01T09:00:00Z", in_menu: true, menu_order: 20 },
  { id: "b", title: "Hello", status: "Published", published_at: "2026-11-05T09:00:00Z", in_menu: false },
  { id: "c", title: "zeta draft", status: "Draft", in_menu: true, menu_order: 1 },
  { id: "d", title: "Alpha draft", status: "Draft" },
  { id: "e", title: "Contact", status: "Published", published_at: "2026-10-01T09:00:00Z", in_menu: true, menu_order: 20 },
];

describe("the website's pages", () => {
  it("lists what is published, newest first, then drafts by title", () => {
    expect(ordered(rows).map((r) => r.id)).toEqual(["b", "a", "e", "d", "c"]);
  });

  it("shows the menu as the site does: published pages in it, by order then title", () => {
    expect(menu(rows).map((r) => r.id)).toEqual(["a", "e"]);
  });
});
