// @vitest-environment jsdom
// The home's templates: "Make a page from it" runs the template's `use_template` operation with the title typed, and
// opens the draft page it made.

import { afterEach, describe, expect, it, vi } from "vitest";
import { button, fakeServer, open, press, settle, type } from "./mount";

afterEach(() => vi.unstubAllGlobals());

describe("templates", () => {
  it("use_template_opens_draft", async () => {
    const s = fakeServer();
    s.answers["/api/v1/WebPage"] = {
      rows: [
        { id: "p1", title: "About", slug: "about", status: "Published", in_menu: true },
        { id: "t1", title: "About Template", slug: "template-about", status: "Draft", is_template: true },
      ],
    };
    s.answers["POST /api/v1/WebPage/t1/actions/use_template"] = () => ({ page: "p9" });
    const app = await open("/");
    // The template is offered, not listed among the pages.
    const listed = app.host.querySelector('[aria-labelledby="ws-pages"] > ul')?.textContent ?? "";
    expect(listed).toContain("About");
    expect(listed).not.toContain("About Template");
    expect(button(app.host, "Make a page from it")?.hasAttribute("disabled")).toBe(true);
    await type(app.host.querySelector("#ws-template-title"), "About Acme");
    await press(button(app.host, "Make a page from it"));
    const sent = s.asked.find((a) => a.method === "POST" && a.path.endsWith("/use_template"));
    expect(sent?.body).toEqual({ title: "About Acme" });
    // The record's page loads lazily: the router moves once it has.
    const at = () => app.router.state.navigation.location?.pathname ?? app.router.state.location.pathname;
    for (let i = 0; i < 100 && at() !== "/e/WebPage/p9"; i++) await settle(1);
    expect(at()).toBe("/e/WebPage/p9");
    await app.close();
  });
});
