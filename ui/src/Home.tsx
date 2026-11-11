// The editors' page: the site's pages and posts, published ones first, the menu as the site shows
// it, and a preview of the one chosen, drafts included, as the site renders it. The public site is
// served on the domain an administrator gives to the Website app.

import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Link } from "react-router";
import { ApiError, api, docPath, t, isolate } from "@ferrum/client";
import { Badge, ListSkeleton, ReaderSkeleton, StatePanel, StatusBadge, useShellTitle, placed } from "@ferrum/kit";
import { menu, ordered, type Row } from "./pages";

// The stored states of a page (website.yaml): Draft, Published.
const STATUS_TONES = { Draft: "neutral", Published: "green" } as const;

// A failed read said why: refused (403) is not shown as an error, no connection (0) is offline.
function Failed({ error, onRetry }: { error: unknown; onRetry: () => void }) {
  const status = error instanceof ApiError ? error.status : -1;
  return <StatePanel kind={status === 403 ? "denied" : status === 0 ? "offline" : "failed"} error={error} onRetry={onRetry} />;
}

export function Home() {
  useShellTitle(t("Website"));
  const [chosen, setChosen] = useState<string | null>(null);
  const q = useQuery({
    queryKey: ["website", "pages"],
    queryFn: () =>
      api
        .get<{ rows: Row[] }>(docPath("WebPage"), { fields: "title,slug,kind,status,in_menu,menu_order,published_at,summary", limit: 500 })
        .then((r) => ordered(r.rows)),
  });
  // The site's own rendering of the page's text, drafts included (`/api/app/website/preview/{id}`).
  const preview = useQuery({
    queryKey: ["website", "preview", chosen],
    enabled: !!chosen,
    queryFn: ({ signal }) => api.get<{ title: string; html: string }>(`/api/app/website/preview/${chosen}`, {}, signal),
  });
  if (q.isPending) return <ListSkeleton />;
  if (q.isError) return <Failed error={q.error} onRetry={() => void q.refetch()} />;
  const site = menu(q.data);
  return (
    <div className="np-page">
      <header className="page-head">
        <div>
          <h1 className="page-title">{t("Website")}</h1>
          <p className="page-sub">
            {placed(t("The menu: {p1} · Blog"), { p1: site.length ? site.map((p) => String(p.title)).join(" · ") : t("no pages in it yet") })}
          </p>
        </div>
        <div className="page-actions">
          <Link className="btn btn-md" to="/e/WebsiteSettings">
            {t("Settings")}
          </Link>
          <Link className="btn btn-primary btn-md" to="/e/WebPage/new">
            {t("New page")}
          </Link>
        </div>
      </header>
      <div className="np-columns">
        <section className="card card-pad stack" aria-labelledby="ws-pages">
          <h2 id="ws-pages" className="font-semibold">{t("Pages and posts ({length})", { length: q.data.length })}</h2>
          {q.data.length === 0 && <p className="faint">{t("No pages yet.")}</p>}
          <ul className="stack">
            {q.data.map((p) => (
              <li key={p.id} className="row" style={{ gap: 8, flexWrap: "wrap" }}>
                <button type="button" className="link-btn" aria-pressed={chosen === p.id} onClick={() => setChosen(p.id)}>
                  {isolate(String(p.title))}
                </button>
                <span className="faint" dir="ltr">/{isolate(String(p.slug ?? ""))}</span>
                <StatusBadge value={p.status} tones={STATUS_TONES} />
                {p.kind === "Post" ? <Badge tone="blue">{t("Post")}</Badge> : null}
                <Link to={`/e/WebPage/${p.id}`}>{t("Edit")}</Link>
              </li>
            ))}
          </ul>
        </section>
        <section className="card card-pad stack" aria-labelledby="ws-preview">
          <h2 id="ws-preview" className="font-semibold">{t("Preview")}</h2>
          {!chosen ? (
            <p className="faint">{t("Choose a page to see it as the site shows it.")}</p>
          ) : preview.isPending ? (
            <ReaderSkeleton />
          ) : preview.isError ? (
            <Failed error={preview.error} onRetry={() => void preview.refetch()} />
          ) : (
            <article className="stack">
              <h3 className="font-semibold">{isolate(preview.data.title)}</h3>
              {/* The server's HTML: its text escaped, raw HTML dropped, and only safe links and https images kept. */}
              <div className="ws-site" dangerouslySetInnerHTML={{ __html: preview.data.html }} />
            </article>
          )}
        </section>
      </div>
    </div>
  );
}
