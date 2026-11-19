// @vitest-environment jsdom
// Without Forms installed the editor offers no Form block and a Form block already there says it shows nothing; with
// Forms, the form's address is typed with the published forms the person can see offered.

import { afterEach, describe, expect, it, vi } from "vitest";
import { fakeServer, open } from "./mount";

afterEach(() => vi.unstubAllGlobals());

const page = { id: "p1", version: 1, title: "Contact", slug: "contact", blocks: [{ kind: "Form", form: "contact" }] };

describe("the form picker", () => {
  it("form_picker_hidden_when_forms_absent", async () => {
    const s = fakeServer({ forms: false });
    s.answers["/api/v1/WebPage/p1"] = page;
    s.answers["/api/app/website/preview/p1"] = { title: "Contact", html: "" };
    const app = await open("/blocks/p1");
    const kinds = [...app.host.querySelectorAll("#ws-add option")].map((o) => o.textContent);
    expect(kinds).not.toContain("Form");
    expect(app.host.textContent).toContain("Forms is not installed: this block shows nothing.");
    expect(app.host.querySelector("datalist")).toBeNull();
    expect(s.asked.some((a) => a.path === "/api/v1/Form")).toBe(false);
    await app.close();
  });

  it("offers the published forms the person can see", async () => {
    const s = fakeServer({ forms: true });
    s.answers["/api/v1/WebPage/p1"] = page;
    s.answers["/api/app/website/preview/p1"] = { title: "Contact", html: "" };
    s.answers["/api/v1/Form"] = { rows: [{ id: "f1", slug: "contact", title: "Contact us" }] };
    const app = await open("/blocks/p1");
    expect([...app.host.querySelectorAll("#ws-add option")].map((o) => o.textContent)).toContain("Form");
    expect([...app.host.querySelectorAll("datalist option")].map((o) => o.getAttribute("value"))).toEqual(["contact"]);
    await app.close();
  });
});
