// @vitest-environment jsdom
// The page editor as mounted at the page's own address, /e/WebPage/{id}: blocks move by their buttons (the keyboard's way; focus stays on the
// button pressed), each shows only the fields of its kind, and saving sends the new order through the record's update.

import { afterEach, describe, expect, it, vi } from "vitest";
import { button, fakeServer, open, press, type } from "./mount";
import { fieldsOf, forSave, move } from "./blocks";

afterEach(() => vi.unstubAllGlobals());

const page = {
  id: "p1",
  version: 3,
  title: "Contact",
  slug: "contact",
  blocks: [
    { kind: "Text", heading: "First", text: "One" },
    { kind: "Form", heading: "Second", form: "contact" },
    { kind: "Image", image: "https://cdn.acme.test/a.png", image_alt: "A van", heading: "Third" },
  ],
};

function server(forms = true) {
  const s = fakeServer({ forms });
  s.answers["/api/v1/WebPage/p1"] = page;
  s.answers["/api/app/website/preview/p1"] = { title: "Contact", html: "<p>One</p>" };
  s.answers["PATCH /api/v1/WebPage/p1"] = () => ({ ...page, version: 4 });
  return s;
}

const order = (host: Element) => [...host.querySelectorAll("ol.ws-blocks > li")].map((li) => li.getAttribute("aria-label"));

describe("the blocks editor", () => {
  it("reorder_by_keyboard: Move down moves the block, keeps focus on it, and Save sends the new order", async () => {
    const s = server();
    const app = await open("/e/WebPage/p1");
    expect(order(app.host)).toEqual(["Block 1: Text", "Block 2: Form", "Block 3: Image"]);
    await press(button(app.host, "Move block 1 down"));
    expect(order(app.host)).toEqual(["Block 1: Form", "Block 2: Text", "Block 3: Image"]);
    expect(document.activeElement?.getAttribute("aria-label")).toBe("Move block 2 down");
    expect(button(app.host, "Move block 1 up")?.hasAttribute("disabled")).toBe(true);
    await press(button(app.host, "Save page"));
    const sent = s.asked.find((a) => a.method === "PATCH");
    expect(sent?.path).toBe("/api/v1/WebPage/p1");
    expect((sent?.body as { blocks: { heading: string }[] }).blocks.map((b) => b.heading)).toEqual(["Second", "First", "Third"]);
    await app.close();
  });

  it("kind_fields_shown: a Form block shows its form's address, not a picture; an Image block its picture", async () => {
    server();
    const app = await open("/e/WebPage/p1");
    const [text, form, image] = [...app.host.querySelectorAll("ol.ws-blocks > li")];
    const labels = (el: Element) => [...el.querySelectorAll("label.field-label")].map((l) => l.textContent?.replace("*", "").trim());
    expect(labels(form)).toEqual(["Heading", "Form's address name"]);
    expect(labels(image)).toEqual(["Picture's address", "What the picture shows", "Heading"]);
    expect(labels(text)).toEqual(["Heading", "Text (Markdown)"]);
    await app.close();
  });
});

describe("the page's own fields", () => {
  it("are edited in the page editor's panel and saved with the blocks; there is no second page for blocks", async () => {
    const s = server();
    const app = await open("/e/WebPage/p1");
    const title = [...app.host.querySelectorAll("label.field-label")].find((l) => l.textContent?.trim() === "Title");
    await type(document.getElementById(title?.getAttribute("for") ?? ""), "Write to us");
    expect(button(app.host, "Publish")?.hasAttribute("disabled")).toBe(true);
    await press(button(app.host, "Save page"));
    const sent = s.asked.find((a) => a.method === "PATCH")?.body as { title: string; blocks: unknown[]; slug: string };
    expect(sent.title).toBe("Write to us");
    expect(sent.slug).toBe("contact");
    expect(sent.blocks).toHaveLength(3);
    await app.close();
    const old = await open("/blocks/p1");
    expect(old.host.textContent).toContain("Page not found");
    await old.close();
  });
});

describe("what the editor saves", () => {
  it("keeps only a kind's own fields, and moves within the list", () => {
    expect(forSave([{ kind: "Form", form: "contact", image: "https://x.test/a.png", heading: "" }])).toEqual([{ kind: "Form", form: "contact" }]);
    expect(move([1, 2, 3], 0, 1)).toEqual([2, 1, 3]);
    expect(move([1, 2, 3], 2, 1)).toEqual([1, 2, 3]);
    expect(fieldsOf("Latest Posts")).toEqual(["heading", "posts"]);
  });
});
