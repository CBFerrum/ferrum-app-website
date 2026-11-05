// The editors' page: the site's pages and posts, published ones first, the menu as the site shows
// it, and a preview of the one chosen, drafts included. The public site itself is served on the
// domain an administrator gives to the Website app.

import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Link } from "react-router";
import { api, docPath } from "@ferrum/client";
import { Badge, ErrorState, Markdown, PageLoading, useShellTitle } from "@ferrum/kit";
import { menu, ordered, type Row } from "./pages";

export function Home() {
  useShellTitle("Website");
  const [chosen, setChosen] = useState<string | null>(null);
  const q = useQuery({
    queryKey: ["website", "pages"],
    queryFn: () =>
      api
        .get<{ rows: Row[] }>(docPath("WebPage"), { fields: "title,slug,kind,status,in_menu,menu_order,published_at,summary", limit: 500 })
        .then((r) => ordered(r.rows)),
  });
  const preview = useQuery({ queryKey: ["website", "page", chosen], enabled: !!chosen, queryFn: () => api.get<Row>(docPath("WebPage", chosen!)) });
  if (q.isPending) return <PageLoading />;
  if (q.isError) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  const site = menu(q.data);
  return (
    <div className="np-page">
      <header className="row" style={{ justifyContent: "space-between", flexWrap: "wrap", gap: 8 }}>
        <h2>Website</h2>
        <span className="row" style={{ gap: 12 }}>
          <Link to="/e/WebsiteSettings">Settings</Link>
          <Link to="/e/WebPage/new">New page</Link>
        </span>
      </header>
      <p className="faint">
        The menu: {site.length ? site.map((p) => String(p.title)).join(" · ") : "no pages in it yet"} · Blog
      </p>
      <div className="np-columns">
        <section className="card card-pad stack" aria-labelledby="ws-pages">
          <h3 id="ws-pages">Pages and posts ({q.data.length})</h3>
          {q.data.length === 0 && <p className="faint">No pages yet.</p>}
          <ul className="stack">
            {q.data.map((p) => (
              <li key={p.id} className="row" style={{ gap: 8, flexWrap: "wrap" }}>
                <button type="button" className="link-btn" aria-pressed={chosen === p.id} onClick={() => setChosen(p.id)}>
                  {String(p.title)}
                </button>
                <span className="faint">/{String(p.slug ?? "")}</span>
                <Badge tone={p.status === "Published" ? "green" : "neutral"}>{String(p.status)}</Badge>
                {p.kind === "Post" ? <Badge tone="blue">Post</Badge> : null}
                <Link to={`/e/WebPage/${p.id}`}>Edit</Link>
              </li>
            ))}
          </ul>
        </section>
        <section className="card card-pad stack" aria-labelledby="ws-preview">
          <h3 id="ws-preview">Preview</h3>
          {!chosen ? (
            <p className="faint">Choose a page to see it as the site shows it.</p>
          ) : preview.isPending ? (
            <PageLoading />
          ) : preview.isError ? (
            <ErrorState error={preview.error} onRetry={() => void preview.refetch()} />
          ) : (
            <article className="stack">
              <h4 className="font-semibold">{String(preview.data.title)}</h4>
              <Markdown source={String(preview.data.body ?? "")} />
            </article>
          )}
        </section>
      </div>
    </div>
  );
}
