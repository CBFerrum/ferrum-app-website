// For the screens' tests: the app's real route table mounted on a fake server (fetch answered from `answers`), with
// what was asked kept, so a test presses what a person presses and reads what they see.

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { createMemoryRouter, RouterProvider } from "react-router";
import { vi } from "vitest";
import { setFrameMode } from "@ferrum/records/frame";
import { routes } from "./routes";

export const me = {
  tenant: "01a1cdeb-1585-7677-ab18-d6cd4cef40f7",
  today: "2026-11-19",
  timezone: "UTC",
  display_timezone: "UTC",
  fiscal_year_start_month: 1,
  default_company: null,
  scopes: {},
  shell_v2: true,
  two_factor_setup: false,
  packages: ["core", "website"],
  user: { id: "01a1cdeb-1592-7353-9e49-69fca11ba7f3", email: "web@acme.test", full_name: "Web Editor", is_admin: false, roles: ["Site Editor"] },
};

const all = { actions: [], cancel: "all", create: "all", delete: "all", export: "all", import: "all", read: "all", submit: "all", write: "all" };
const summary = (name: string, pkg = "website") => ({ custom: false, description: null, icon: null, kind: "master", label: name, label_plural: `${name}s`, module: pkg, name, package: pkg, permissions: all, submittable: false });

/** What the server answers (a function answers with its result), what was asked, and what was sent. */
export interface Server {
  answers: Record<string, unknown | ((body: unknown) => unknown)>;
  asked: { method: string; path: string; body: unknown }[];
}

export function fakeServer(o: { forms?: boolean } = {}): Server {
  const index = {
    entities: [summary("WebPage"), summary("WebRedirect"), summary("WebsiteSettings"), ...(o.forms ? [summary("Form", "forms")] : [])],
    modules: [{ icon: "file-text", items: ["WebPage", "WebRedirect", "WebsiteSettings"], label: "Website", name: "website", order: 59, package: "website" }],
    packages: ["core", "website", ...(o.forms ? ["forms"] : [])],
    version: 1,
    warnings: [],
  };
  const s: Server = { answers: { "/api/auth/me": me, "/api/meta": index }, asked: [] };
  (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  (globalThis as { matchMedia?: unknown }).matchMedia = () => ({ matches: false, addEventListener() {}, removeEventListener() {} });
  (globalThis as { requestAnimationFrame?: unknown }).requestAnimationFrame = (f: () => void) => setTimeout(f, 0);
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = new URL(String(input instanceof Request ? input.url : input), "http://localhost");
      const method = (init?.method ?? (input instanceof Request ? input.method : "GET")).toUpperCase();
      const raw = init?.body;
      const body = typeof raw === "string" ? JSON.parse(raw) : raw;
      s.asked.push({ method, path: url.pathname, body });
      const json = (b: unknown, status = 200) => new Response(JSON.stringify(b), { status, headers: { "content-type": "application/json" } });
      const a = s.answers[`${method} ${url.pathname}`] ?? s.answers[url.pathname];
      if (typeof a === "function") return json(a(body));
      if (a !== undefined) return json(a);
      return url.pathname.startsWith("/api/v1/") ? json({ rows: [], total: 0 }) : json({ error: "not found" }, 404);
    }),
  );
  setFrameMode("app", ["website"]);
  return s;
}

/** Waits for the screen to settle (queries answered, lazy pages loaded). */
export async function settle(times = 30) {
  for (let i = 0; i < times; i++) await act(async () => void (await new Promise((r) => setTimeout(r, 15))));
}

/** The app at `path`: its element, its router, and a way to take it down. */
export async function open(path: string) {
  const host = document.createElement("div");
  document.body.append(host);
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const router = createMemoryRouter(routes, { initialEntries: [path] });
  const root = createRoot(host);
  await act(async () => {
    root.render(
      <QueryClientProvider client={qc}>
        <RouterProvider router={router} />
      </QueryClientProvider>,
    );
  });
  // Until the page itself is drawn (its lazy module loaded and its first answers in), at most about three seconds.
  for (let i = 0; i < 200 && !host.querySelector("main")?.textContent?.trim(); i++) await settle(1);
  await settle(10);
  return {
    host,
    router,
    close: async () => {
      await act(async () => root.unmount());
      host.remove();
    },
  };
}

/** Presses `el` as a person does, and lets the screen answer. */
export async function press(el: Element | null | undefined) {
  if (!el) throw new Error("nothing to press");
  await act(async () => (el as HTMLElement).click());
  await settle(20);
}

/** Types `value` into an input as a person does. */
export async function type(el: Element | null | undefined, value: string) {
  const input = el as HTMLInputElement;
  if (!input) throw new Error("nothing to type into");
  const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
  await act(async () => {
    set?.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await settle(3);
}

export const button = (host: Element, name: string) => [...host.querySelectorAll("button")].find((b) => b.getAttribute("aria-label") === name || b.textContent?.trim() === name);
