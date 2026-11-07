// The editors' page: the site's pages and posts, published ones first, the menu as the site shows
// it, and a preview of the one chosen, drafts included, as the site renders it. The public site is
// served on the domain an administrator gives to the Website app.

import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Link } from "react-router";
import { api, docPath, t, isolate } from "@ferrum/client";
import { Badge, ErrorState, PageLoading, useShellTitle } from "@ferrum/kit";
import { menu, ordered, type Row } from "./pages";

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
  if (q.isPending) return <PageLoading />;
  if (q.isError) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  const site = menu(q.data);
  return (
    <div className="np-page">
      <header className="row" style={{ justifyContent: "space-between", flexWrap: "wrap", gap: 8 }}>
        <h2>{t("Website")}</h2>
        <span className="row" style={{ gap: 12 }}>
          <Link to="/e/WebsiteSettings">{t("Settings")}</Link>
          <Link to="/e/WebPage/new">{t("New page")}</Link>
        </span>
      </header>
      <p className="faint">
        {t("The menu:")} {site.length ? site.map((p) => String(p.title)).join(" · ") : t("no pages in it yet")} {t("· Blog")}
      </p>
      <div className="np-columns">
        <section className="card card-pad stack" aria-labelledby="ws-pages">
          <h3 id="ws-pages">{t("Pages and posts ({length})", { length: q.data.length })}</h3>
          {q.data.length === 0 && <p className="faint">{t("No pages yet.")}</p>}
          <ul className="stack">
            {q.data.map((p) => (
              <li key={p.id} className="row" style={{ gap: 8, flexWrap: "wrap" }}>
                <button type="button" className="link-btn" aria-pressed={chosen === p.id} onClick={() => setChosen(p.id)}>
                  {isolate(String(p.title))}
                </button>
                <span className="faint">/{isolate(String(p.slug ?? ""))}</span>
                <Badge tone={p.status === "Published" ? "green" : "neutral"}>{isolate(String(p.status))}</Badge>
                {p.kind === "Post" ? <Badge tone="blue">{t("Post")}</Badge> : null}
                <Link to={`/e/WebPage/${p.id}`}>{t("Edit")}</Link>
              </li>
            ))}
          </ul>
        </section>
        <section className="card card-pad stack" aria-labelledby="ws-preview">
          <h3 id="ws-preview">{t("Preview")}</h3>
          {!chosen ? (
            <p className="faint">{t("Choose a page to see it as the site shows it.")}</p>
          ) : preview.isPending ? (
            <PageLoading />
          ) : preview.isError ? (
            <ErrorState error={preview.error} onRetry={() => void preview.refetch()} />
          ) : (
            <article className="stack">
              <h4 className="font-semibold">{isolate(preview.data.title)}</h4>
              {/* The server's HTML: its text escaped, raw HTML dropped, and only safe links and https images kept. */}
              <div className="ws-site" dangerouslySetInnerHTML={{ __html: preview.data.html }} />
            </article>
          )}
        </section>
      </div>
    </div>
  );
}
