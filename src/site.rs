//! The public site. On a `site:website` host the platform hands every path to `/_site/{path}`
//! with the site's own origin (`x-ferrum-site-origin`, never the caller's); the pages are rendered
//! here, on the server, with no script:
//!
//! * `/`: the home page the settings name, else the latest posts;
//! * `/blog`: the posts, newest first; `/{slug}`: a published page or post, its text then its blocks;
//! * an address no page has: the redirect from it, one step, else not found;
//! * `/sitemap.xml`, `/robots.txt`, and `/assets/site.css`.
//!
//! Drafts and templates are never served. Text is escaped; Markdown goes through [`markdown`]; an
//! image, a button or a form frame is drawn only from an address [`image_ok`], [`safe_href`] or
//! [`form_ok`] takes. Each page served to a person (not a crawler) adds one to its day's count in
//! `website_view`. Signed in, whoever may read a page gets it as the site renders it, drafts
//! included, at `/api/app/website/preview/{id}` (the editor's preview).

use crate::pages::slugify;
use ferrum_domain::util::{b, id, rows, s};
use ferrum_kernel::brand::PublicBrand;
use ferrum_kernel::{Ctx, Doc, Error, Filter, Kernel, Result, Site, SiteRequest, SiteResponse, Tx};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde_json::{Value, json};
use uuid::Uuid;

const CSS: &str = include_str!("../assets/site.css");
/// Posts on the blog and the home page at most.
const POSTS: u32 = 50;
/// Entries of the menu at most (pages and links, with the pages under them).
const MENU: usize = 60;
/// The longest address looked up among the redirects.
pub(crate) const PATH_MAX: usize = 300;

pub struct WebsiteSite;

#[async_trait::async_trait]
impl Site for WebsiteSite {
    /// The site's pages at the root of its `site:website` host, so a domain can be given to it.
    fn serves_pages(&self) -> bool {
        true
    }

    async fn handle(&self, k: &Kernel, req: SiteRequest) -> Result<SiteResponse> {
        let path = req.path.trim_start_matches('/');
        if let Some(id) = path.strip_prefix("preview/") {
            return preview(k, &req, id).await;
        }
        let Some(rest) = path.strip_prefix("_site").filter(|r| r.is_empty() || r.starts_with('/')) else {
            return Err(Error::NotFound(format!("website endpoint {} {}", req.method, req.path)));
        };
        if req.method != "GET" {
            return Err(Error::NotFound("the site is read only".into()));
        }
        let origin = req.headers.get("x-ferrum-site-origin").and_then(Value::as_str).unwrap_or_default().trim_end_matches('/').to_string();
        let agent = req.headers.get("user-agent").and_then(Value::as_str).unwrap_or_default();
        let ctx = Ctx::system(req.tenant, k.registry.schema(req.tenant).await?);
        let mut tx = k.db.tenant_tx(req.tenant).await?;
        let out = page(k, &ctx, &mut tx, rest.trim_matches('/'), &origin, !crawler(agent)).await;
        tx.commit().await?;
        out
    }
}

/// A crawler or a link preview (`User-Agent` naming itself so): served, never counted.
pub(crate) fn crawler(agent: &str) -> bool {
    let a = agent.to_ascii_lowercase();
    ["bot", "crawl", "spider", "preview"].iter().any(|w| a.contains(w))
}

/// `GET preview/{id}`: a page or post, drafts included, as the site renders its text and blocks, for
/// whoever may read it with their own rights: `{title, html}`.
async fn preview(k: &Kernel, req: &SiteRequest, id: &str) -> Result<SiteResponse> {
    let user = req.user.as_ref().ok_or_else(|| Error::Unauthorized("sign in first".into()))?;
    if req.method != "GET" {
        return Err(Error::NotFound("the preview is read only".into()));
    }
    let id: Uuid = id.parse().map_err(|_| Error::NotFound("page".into()))?;
    let page = k.get(user, "WebPage", id, false).await?;
    // What the blocks show besides the page (the latest posts, the workspace's address name) is the site's, public.
    let ctx = Ctx::system(user.tenant, user.schema.clone());
    let mut tx = k.db.tenant_tx(user.tenant).await?;
    let ws = workspace(&mut tx, user.tenant).await?;
    // In the editor the form's page is on the editor's own address.
    let form_base = ctx.schema.entity("Form").map(|_| String::new());
    let body = format!("{}{}", markdown(s(&page, "body").unwrap_or_default()), blocks(k, &ctx, &mut tx, &ws, &page, form_base).await?);
    tx.commit().await?;
    let mut r = SiteResponse::json(200, &json!({"title": s(&page, "title"), "html": body}));
    r.headers.push(("cache-control".into(), "no-store".into()));
    Ok(r)
}

/// The workspace a site is of: its address name, its name and its language.
pub(crate) struct Workspace {
    pub slug: String,
    pub name: String,
    pub language: String,
}

pub(crate) async fn workspace(tx: &mut Tx, tenant: Uuid) -> Result<Workspace> {
    let (slug, name, language): (String, String, Option<String>) =
        sqlx::query_as("select slug, name, settings->>'language' from tenant where id = $1").bind(tenant).fetch_one(&mut **tx).await?;
    let language = language.filter(|l| ferrum_kernel::tenant::valid_language(l)).unwrap_or_else(|| "en".into());
    Ok(Workspace { slug, name, language })
}

/// One entry of the menu: where it goes, its words, and the pages under it.
struct Item {
    href: String,
    label: String,
    under: Vec<(String, String)>,
}

/// The site's name, words, menu and look, from its settings, its published pages and its brand.
struct Info {
    ws: Workspace,
    title: String,
    tagline: String,
    blog: String,
    footer: String,
    menu: Vec<Item>,
    layout: String,
    brand: Option<PublicBrand>,
}

/// The settings record with its tables (`find_tx` leaves tables out).
async fn settings(k: &Kernel, ctx: &Ctx, tx: &mut Tx) -> Result<Doc> {
    match k.find_tx(ctx, tx, "WebsiteSettings", &[], None, 1).await?.into_iter().next().and_then(|d| id(&d, "id")) {
        Some(sid) => k.get_tx(ctx, tx, "WebsiteSettings", sid, false).await,
        None => Ok(Doc::new()),
    }
}

async fn info(k: &Kernel, ctx: &Ctx, tx: &mut Tx, origin: &str) -> Result<Info> {
    let settings = settings(k, ctx, tx).await?;
    let text = |f: &str, default: &str| s(&settings, f).unwrap_or(default).to_string();
    let ws = workspace(tx, ctx.tenant).await?;
    let pages: Vec<Doc> = k
        .find_tx(ctx, tx, "WebPage", &[Filter::eq("status", "Published"), Filter::eq("in_menu", true)], Some("menu_order asc"), 200)
        .await?
        .into_iter()
        // A translation is reached through the page it translates; a template is never served.
        .filter(|p| !b(p, "is_template") && id(p, "translation_of").is_none())
        .collect();
    let order = |p: &Doc| (p.get("menu_order").and_then(Value::as_i64).unwrap_or(100), s(p, "title").unwrap_or_default().to_lowercase());
    let mut top: Vec<(i64, String, Item, Option<Uuid>)> = Vec::new();
    for p in pages.iter().filter(|p| id(p, "menu_parent").is_none()) {
        let (Some(slug), Some(title)) = (s(p, "slug"), s(p, "title")) else { continue };
        let (n, key) = order(p);
        top.push((n, key, Item { href: format!("/{slug}"), label: title.to_string(), under: vec![] }, id(p, "id")));
    }
    for link in rows(&settings, "menu_links") {
        let (Some(url), Some(label)) = (s(&link, "url"), s(&link, "label")) else { continue };
        if safe_href(url) {
            let n = link.get("menu_order").and_then(Value::as_i64).unwrap_or(100);
            top.push((n, label.to_lowercase(), Item { href: url.to_string(), label: label.to_string(), under: vec![] }, None));
        }
    }
    let mut under: Vec<&Doc> = pages.iter().filter(|p| id(p, "menu_parent").is_some()).collect();
    under.sort_by_key(|p| order(p));
    for p in under {
        // Only under a page the menu shows: a page under one taken back is not shown.
        let parent = id(p, "menu_parent");
        if let (Some(t), Some(slug), Some(title)) = (top.iter_mut().find(|t| t.3.is_some() && t.3 == parent), s(p, "slug"), s(p, "title")) {
            t.2.under.push((format!("/{slug}"), title.to_string()));
        }
    }
    top.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    let mut room = MENU;
    let menu = top
        .into_iter()
        .map_while(|(_, _, mut item, _)| {
            room = room.checked_sub(1)?;
            item.under.truncate(room);
            room -= item.under.len();
            Some(item)
        })
        .collect();
    // The brand the settings name, else the address's, else the workspace's default.
    let host = origin.split_once("://").map(|(_, h)| h).filter(|h| !h.is_empty());
    let host_brand = match host {
        Some(h) => k.host_brand(h).await?,
        None => None,
    };
    let brand = k.public_brand(ctx.tenant, id(&settings, "brand"), host_brand).await?;
    Ok(Info {
        title: s(&settings, "site_title").map(str::trim).filter(|t| !t.is_empty()).unwrap_or(&ws.name).to_string(),
        tagline: text("tagline", ""),
        blog: text("blog_title", "Blog"),
        footer: text("footer", ""),
        menu,
        layout: match s(&settings, "layout") {
            Some("Centred") => "centred",
            Some("Wide") => "wide",
            _ => "classic",
        }
        .into(),
        brand,
        ws,
    })
}

/// A published page that is not a template, with its blocks.
async fn published(k: &Kernel, ctx: &Ctx, tx: &mut Tx, filter: Filter) -> Result<Option<Doc>> {
    let hit = k.find_tx(ctx, tx, "WebPage", &[filter, Filter::eq("status", "Published")], None, 1).await?.into_iter().next();
    match hit.filter(|p| !b(p, "is_template")).and_then(|p| id(&p, "id")) {
        Some(pid) => Ok(Some(k.get_tx(ctx, tx, "WebPage", pid, false).await?)),
        None => Ok(None),
    }
}

async fn page(k: &Kernel, ctx: &Ctx, tx: &mut Tx, path: &str, origin: &str, count: bool) -> Result<SiteResponse> {
    match path {
        "assets/site.css" => {
            let mut r = SiteResponse::text(200, "text/css; charset=utf-8", CSS);
            r.headers.push(("cache-control".into(), "public, max-age=31536000, immutable".into()));
            return Ok(r);
        }
        "robots.txt" => return Ok(SiteResponse::text(200, "text/plain; charset=utf-8", format!("User-agent: *\nAllow: /\nSitemap: {origin}/sitemap.xml\n"))),
        "sitemap.xml" => {
            let pages = k.find_tx(ctx, tx, "WebPage", &[Filter::eq("status", "Published")], Some("published_at desc"), 5000).await?;
            let mut xml = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><url><loc>{0}/</loc></url><url><loc>{0}/blog</loc></url>"#,
                esc(origin)
            );
            // Hidden from search engines and templates are not listed.
            for p in pages.iter().filter(|p| !b(p, "noindex") && !b(p, "is_template")) {
                let day = s(p, "published_at").map(|t| t.chars().take(10).collect::<String>()).unwrap_or_default();
                xml.push_str(&format!("<url><loc>{}/{}</loc><lastmod>{}</lastmod></url>", esc(origin), esc(s(p, "slug").unwrap_or_default()), esc(&day)));
            }
            xml.push_str("</urlset>");
            return Ok(SiteResponse::text(200, "application/xml; charset=utf-8", xml));
        }
        _ => {}
    }
    let doc = match path {
        "" => match k.find_tx(ctx, tx, "WebsiteSettings", &[], None, 1).await?.into_iter().next().and_then(|d| id(&d, "home_page")) {
            Some(home) => published(k, ctx, tx, Filter::eq("id", home.to_string())).await?,
            None => None,
        },
        "blog" => None,
        slug if slugify(slug) == slug => match published(k, ctx, tx, Filter::eq("slug", slug)).await? {
            Some(p) => Some(p),
            None => return redirect(k, ctx, tx, path).await,
        },
        _ => return redirect(k, ctx, tx, path).await,
    };
    let site = info(k, ctx, tx, origin).await?;
    let scheme = if origin.starts_with("http://") { "http" } else { "https" };
    let served = match &doc {
        Some(p) => format!("/{}", s(p, "slug").unwrap_or_default()),
        None if path == "blog" => "/blog".into(),
        None => "/".into(),
    };
    let html = match doc {
        Some(p) => {
            let title = s(&p, "title").unwrap_or_default();
            let when = (s(&p, "kind") == Some("Post"))
                .then(|| {
                    s(&p, "published_at")
                        .map(|t| format!(r#"<p class="when"><time datetime="{0}">{0}</time></p>"#, esc(&t.chars().take(10).collect::<String>())))
                })
                .flatten()
                .unwrap_or_default();
            let form_base = form_base(k, ctx, scheme).await?;
            let main = format!(
                "<article><h1>{}</h1>{when}{}{}</article>",
                esc(title),
                markdown(s(&p, "body").unwrap_or_default()),
                blocks(k, ctx, tx, &site.ws, &p, form_base).await?
            );
            let meta = meta_of(k, ctx, tx, &site, origin, &served, &p).await?;
            layout(&site, origin, &meta, &main)
        }
        None => {
            let posts = latest(k, ctx, tx, POSTS).await?;
            let (heading, description) = if path == "blog" { (site.blog.clone(), site.tagline.clone()) } else { (site.title.clone(), site.tagline.clone()) };
            let meta = Meta { path: served.clone(), title: heading.clone(), description, lang: site.ws.language.clone(), ..Meta::default() };
            layout(&site, origin, &meta, &format!("<h1>{}</h1>{}", esc(&heading), post_list(&posts)))
        }
    };
    if count {
        // One more view of this address today (the workspace's day). Only addresses that answer a page are counted, so
        // the rows stay at most one per page and day however many requests come.
        sqlx::query(
            "insert into website_view (tenant_id, path, day, views) values ($1, $2, (now() at time zone $3)::date, 1) \
             on conflict (tenant_id, path, day) do update set views = website_view.views + 1",
        )
        .bind(ctx.tenant)
        .bind(&served)
        .bind(&ctx.schema.timezone)
        .execute(&mut **tx)
        .await?;
    }
    Ok(SiteResponse::text(200, "text/html; charset=utf-8", html))
}

/// An address no page has: the redirect from it (permanent 301 or for now 302), once, never followed further here;
/// else not found.
async fn redirect(k: &Kernel, ctx: &Ctx, tx: &mut Tx, path: &str) -> Result<SiteResponse> {
    let not_found = || Error::NotFound("page".into());
    if path.is_empty() || path.len() > PATH_MAX {
        return Err(not_found());
    }
    let from = format!("/{path}");
    let hit = k.find_tx(ctx, tx, "WebRedirect", &[Filter::eq("from_path", from.as_str())], None, 1).await?.into_iter().next().ok_or_else(not_found)?;
    let to = s(&hit, "to").filter(|t| crate::redirects::target_ok(t)).ok_or_else(not_found)?;
    let permanent = b(&hit, "permanent");
    let mut r = SiteResponse::text(if permanent { 301 } else { 302 }, "text/plain; charset=utf-8", "");
    r.headers.push(("location".into(), to.to_string()));
    r.headers.push(("cache-control".into(), if permanent { "public, max-age=3600" } else { "no-store" }.into()));
    Ok(r)
}

/// The newest published posts that are not templates.
async fn latest(k: &Kernel, ctx: &Ctx, tx: &mut Tx, n: u32) -> Result<Vec<Doc>> {
    let mut posts = k.find_tx(ctx, tx, "WebPage", &[Filter::eq("status", "Published"), Filter::eq("kind", "Post")], Some("published_at desc"), n + 5).await?;
    posts.retain(|p| !b(p, "is_template"));
    posts.truncate(n as usize);
    Ok(posts)
}

fn post_list(posts: &[Doc]) -> String {
    let items: String = posts
        .iter()
        .map(|p| {
            let day = s(p, "published_at").map(|t| t.chars().take(10).collect::<String>()).unwrap_or_default();
            let summary = s(p, "summary").map(|t| format!("<p>{}</p>", esc(t))).unwrap_or_default();
            format!(
                r#"<li><a href="/{}">{}</a> <time datetime="{2}">{2}</time>{summary}</li>"#,
                esc(s(p, "slug").unwrap_or_default()),
                esc(s(p, "title").unwrap_or_default()),
                esc(&day)
            )
        })
        .collect();
    if items.is_empty() { r#"<p class="muted">Nothing here yet.</p>"#.to_string() } else { format!(r#"<ul class="posts">{items}</ul>"#) }
}

/// Where a Form block's frame takes the form from: the workspace's main staff address, which every public page is on
/// and which lets the workspace's own addresses frame it (BD-80). `None`: Forms is not installed, or no address.
async fn form_base(k: &Kernel, ctx: &Ctx, scheme: &str) -> Result<Option<String>> {
    if ctx.schema.entity("Form").is_none() {
        return Ok(None);
    }
    Ok(k.serving_hosts(ctx.tenant, "shell").await?.into_iter().next().map(|h| format!("{scheme}://{h}")))
}

/// A page's blocks, in order. `form_base`: where a Form block's frame points (`Some("")`: the same address, the
/// editor's preview; `None`: Form blocks draw nothing).
pub(crate) async fn blocks(k: &Kernel, ctx: &Ctx, tx: &mut Tx, ws: &Workspace, page: &Doc, form_base: Option<String>) -> Result<String> {
    let mut out = String::new();
    for bl in rows(page, "blocks") {
        let heading = s(&bl, "heading").map(str::trim).filter(|h| !h.is_empty());
        let h2 = heading.map(|h| format!("<h2>{}</h2>", esc(h))).unwrap_or_default();
        let text = s(&bl, "text").map(markdown).unwrap_or_default();
        let image = || {
            s(&bl, "image")
                .filter(|u| image_ok(u, &ws.slug))
                .map(|u| format!(r#"<img src="{}" alt="{}" loading="lazy">"#, esc(u), esc(s(&bl, "image_alt").unwrap_or_default())))
        };
        let button = || match (s(&bl, "button_label").map(str::trim).filter(|l| !l.is_empty()), s(&bl, "button_url").filter(|u| safe_href(u))) {
            (Some(label), Some(url)) => format!(r#"<p><a class="button" href="{}">{}</a></p>"#, esc(url), esc(label)),
            _ => String::new(),
        };
        let block = match s(&bl, "kind").unwrap_or_default() {
            "Text" => format!(r#"<section class="block block-text">{h2}{text}</section>"#),
            "Image" => match image() {
                Some(img) => {
                    let caption = heading.map(|h| format!("<figcaption>{}</figcaption>", esc(h))).unwrap_or_default();
                    format!(r#"<figure class="block block-image">{img}{caption}</figure>"#)
                }
                None => String::new(),
            },
            "Hero" => format!(r#"<section class="block block-hero">{h2}{text}{}{}</section>"#, image().unwrap_or_default(), button()),
            "Call to Action" => format!(r#"<section class="block block-cta">{h2}{text}{}</section>"#, button()),
            "Form" => match (&form_base, s(&bl, "form").filter(|f| form_ok(f))) {
                (Some(base), Some(form)) => format!(
                    r#"<section class="block block-form">{h2}<iframe class="form-frame" src="{}" title="{}" loading="lazy" data-ferrum-embed></iframe></section>"#,
                    esc(&format!("{base}/forms/f/{}/{form}?embed=1", ws.slug)),
                    esc(heading.unwrap_or(form))
                ),
                _ => String::new(),
            },
            "Latest Posts" => {
                let n = bl.get("posts").and_then(Value::as_u64).unwrap_or(5).clamp(1, 20) as u32;
                format!(r#"<section class="block block-posts">{h2}{}</section>"#, post_list(&latest(k, ctx, tx, n).await?))
            }
            _ => String::new(),
        };
        out.push_str(&block);
    }
    Ok(out)
}

/// What a page's `<head>` says about it.
#[derive(Default)]
struct Meta {
    path: String,
    title: String,
    seo_title: Option<String>,
    description: String,
    lang: String,
    noindex: bool,
    canonical: Option<String>,
    image: Option<String>,
    /// Its translations' (and its own) language and address, when it has any.
    alternates: Vec<(String, String)>,
}

async fn meta_of(k: &Kernel, ctx: &Ctx, tx: &mut Tx, site: &Info, origin: &str, path: &str, p: &Doc) -> Result<Meta> {
    let lang = s(p, "language").filter(|l| ferrum_kernel::tenant::valid_language(l)).unwrap_or(&site.ws.language).to_string();
    // The page and its translations: the page it translates and every translation of that one.
    let mut alternates = Vec::new();
    if let Some(root) = id(p, "translation_of").or_else(|| id(p, "id")) {
        let mut group = k.find_tx(ctx, tx, "WebPage", &[Filter::eq("translation_of", root.to_string()), Filter::eq("status", "Published")], None, 50).await?;
        if !group.is_empty() {
            group.extend(k.find_tx(ctx, tx, "WebPage", &[Filter::eq("id", root.to_string()), Filter::eq("status", "Published")], None, 1).await?);
            for m in group.iter().filter(|m| !b(m, "noindex") && !b(m, "is_template")) {
                if let (Some(l), Some(slug)) = (s(m, "language").filter(|l| ferrum_kernel::tenant::valid_language(l)), s(m, "slug")) {
                    alternates.push((l.to_string(), format!("{origin}/{slug}")));
                }
            }
            alternates.sort();
        }
    }
    let image = s(p, "og_image").filter(|u| image_ok(u, &site.ws.slug)).map(|u| if u.starts_with('/') { format!("{origin}{u}") } else { u.to_string() });
    Ok(Meta {
        path: path.to_string(),
        title: s(p, "title").unwrap_or_default().to_string(),
        seo_title: s(p, "seo_title").map(str::trim).filter(|t| !t.is_empty()).map(str::to_string),
        description: s(p, "summary").unwrap_or_default().to_string(),
        lang,
        noindex: b(p, "noindex"),
        canonical: s(p, "canonical_url").filter(|u| u.starts_with("https://") && safe_href(u)).map(str::to_string),
        image,
        alternates,
    })
}

fn layout(site: &Info, origin: &str, m: &Meta, main: &str) -> String {
    let shown = m.seo_title.as_deref().unwrap_or(&m.title);
    let full = if shown.is_empty() || shown == site.title { esc(&site.title) } else { format!("{} · {}", esc(shown), esc(&site.title)) };
    // What link previews read: the page's title and summary, the site's name, and its address.
    let mut og = format!(
        r#"<meta property="og:type" content="website"><meta property="og:site_name" content="{}"><meta property="og:title" content="{}">"#,
        esc(&site.title),
        esc(if m.title.is_empty() { &site.title } else { &m.title })
    );
    if !m.description.is_empty() {
        og.push_str(&format!(r#"<meta property="og:description" content="{}">"#, esc(&m.description)));
    }
    if !origin.is_empty() {
        og.push_str(&format!(r#"<meta property="og:url" content="{}{}">"#, esc(origin), esc(&m.path)));
    }
    if let Some(img) = &m.image {
        og.push_str(&format!(r#"<meta property="og:image" content="{}"><meta name="twitter:card" content="summary_large_image">"#, esc(img)));
    }
    let description = if m.description.is_empty() { String::new() } else { format!(r#"<meta name="description" content="{}">"#, esc(&m.description)) };
    let robots = if m.noindex { r#"<meta name="robots" content="noindex">"# } else { "" };
    let canonical = match (&m.canonical, origin.is_empty()) {
        (Some(c), _) => format!(r#"<link rel="canonical" href="{}">"#, esc(c)),
        (None, false) => format!(r#"<link rel="canonical" href="{}{}">"#, esc(origin), esc(&m.path)),
        (None, true) => String::new(),
    };
    let hreflang: String = m.alternates.iter().map(|(l, href)| format!(r#"<link rel="alternate" hreflang="{}" href="{}">"#, esc(l), esc(href))).collect();
    let menu: String = site
        .menu
        .iter()
        .map(|i| {
            let link = format!(r#"<a href="{}">{}</a>"#, esc(&i.href), esc(&i.label));
            if i.under.is_empty() {
                return format!("<li>{link}</li>");
            }
            let under: String = i.under.iter().map(|(href, t)| format!(r#"<li><a href="{}">{}</a></li>"#, esc(href), esc(t))).collect();
            format!("<li><details><summary>{}</summary><ul><li>{link}</li>{under}</ul></details></li>", esc(&i.label))
        })
        .collect();
    let tagline = if site.tagline.is_empty() { String::new() } else { format!(r#"<p class="tagline">{}</p>"#, esc(&site.tagline)) };
    let ws = &site.ws.slug;
    let brand_head = site.brand.as_ref().map(|b| b.head(ws)).unwrap_or_default();
    let logo = site.brand.as_ref().and_then(|b| b.file_href(ws, "logo").map(|l| (l, b.file_href(ws, "logo_dark"))));
    let mark = match logo {
        Some((light, dark)) => {
            let dark = dark.map(|d| format!(r#"<source media="(prefers-color-scheme: dark)" srcset="{}">"#, esc(&d))).unwrap_or_default();
            format!(r#"<picture>{dark}<img src="{}" alt="{}"></picture>"#, esc(&light), esc(&site.title))
        }
        None => esc(&site.title),
    };
    let footer_text = match (site.footer.is_empty(), site.brand.as_ref().map(|b| b.footer_text.as_str()).filter(|t| !t.is_empty())) {
        (false, _) => esc(&site.footer),
        (true, Some(t)) => esc(t),
        (true, None) => format!("© {}", esc(&site.title)),
    };
    let links: String =
        site.brand.iter().flat_map(|b| b.links.iter()).map(|(label, url)| format!(r#"<li><a href="{}">{}</a></li>"#, esc(url), esc(label))).collect();
    let links = if links.is_empty() { links } else { format!(r#"<ul class="links">{links}</ul>"#) };
    let dir = if ferrum_kernel::layout::rtl(&m.lang) { "rtl" } else { "ltr" };
    format!(
        r##"<!doctype html><html lang="{lang}" dir="{dir}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>{full}</title>{description}{robots}{canonical}{hreflang}{og}<link rel="stylesheet" href="/assets/site.css?v={v}">{brand_head}</head><body class="layout-{layout}"><a class="skip" href="#main">Skip to content</a><header class="top"><a class="brand" href="/">{mark}</a>{tagline}<nav aria-label="Site"><ul>{menu}<li><a href="/blog">{blog}</a></li></ul></nav></header><main id="main">{main}</main><footer>{footer_text}{links}</footer></body></html>"##,
        lang = esc(&m.lang),
        v = CSS.len(),
        layout = site.layout,
        blog = esc(&site.blog),
    )
}

/// Text for HTML, attributes included.
pub(crate) fn esc(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}

/// Where a link may go: the web, mail, or this site's own paths (not another site's by `//` or `/\`).
pub(crate) fn safe_href(url: &str) -> bool {
    let plain = !url.chars().any(|c| c.is_whitespace() || c.is_control() || c == '\\');
    plain
        && (url.starts_with("https://")
            || url.starts_with("http://")
            || url.starts_with("mailto:")
            || url.starts_with('#')
            || (url.starts_with('/') && !url.starts_with("//")))
}

/// A picture a page may show: a public file of the platform (`/files/public/{id}`), this workspace's DAM web address
/// (`/api/site/{workspace}/dam/w/{id}`), or an `https://` address.
pub(crate) fn image_ok(url: &str, workspace: &str) -> bool {
    let uuid_after = |prefix: &str| url.strip_prefix(prefix).is_some_and(|rest| rest.parse::<Uuid>().is_ok());
    uuid_after("/files/public/") || uuid_after(&format!("/api/site/{workspace}/dam/w/")) || (url.starts_with("https://") && safe_href(url))
}

/// A Forms form's address name as a frame may name it.
pub(crate) fn form_ok(slug: &str) -> bool {
    (1..=80).contains(&slug.len()) && slug.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}

/// A page's Markdown as HTML: tables and strikethrough; raw HTML never passes (its text is
/// dropped), a link that goes elsewhere than [`safe_href`] keeps only its text, and an image is
/// shown only from an `https://` address (otherwise its description stays as text).
pub(crate) fn markdown(source: &str) -> String {
    // For each open link and image: whether its start was kept.
    let mut open: Vec<bool> = Vec::new();
    let events = Parser::new_ext(source, Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH).filter_map(|e| match e {
        Event::Html(_) | Event::InlineHtml(_) | Event::Start(Tag::HtmlBlock) | Event::End(TagEnd::HtmlBlock) => None,
        Event::Start(Tag::Link { link_type, dest_url, title, id }) => {
            let ok = safe_href(&dest_url);
            open.push(ok);
            ok.then_some(Event::Start(Tag::Link { link_type, dest_url, title, id }))
        }
        Event::Start(Tag::Image { link_type, dest_url, title, id }) => {
            let ok = dest_url.starts_with("https://") && safe_href(&dest_url);
            open.push(ok);
            ok.then_some(Event::Start(Tag::Image { link_type, dest_url, title, id }))
        }
        Event::End(TagEnd::Link) | Event::End(TagEnd::Image) => open.pop().unwrap_or(false).then_some(e),
        other => Some(other),
    });
    let mut out = String::with_capacity(source.len() + source.len() / 4);
    pulldown_cmark::html::push_html(&mut out, events);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_ok_takes_three_shapes() {
        let id = "0190f3a0-0000-7000-8000-000000000001";
        assert!(image_ok(&format!("/files/public/{id}"), "acme"));
        assert!(image_ok(&format!("/api/site/acme/dam/w/{id}"), "acme"));
        assert!(image_ok("https://cdn.acme.test/a.png", "acme"));
        for bad in [
            format!("/api/site/other/dam/w/{id}"),
            "/files/public/not-an-id".into(),
            format!("/files/{id}"),
            "//evil.test/a.png".into(),
            "http://acme.test/a.png".into(),
            "data:image/png;base64,AA".into(),
            "https://x.test/a b.png".into(),
            format!("/files/public/{id}/../x"),
        ] {
            assert!(!image_ok(&bad, "acme"), "{bad}");
        }
    }

    #[test]
    fn links_never_leave_by_a_backslash() {
        assert!(!safe_href("/\\evil.test") && !safe_href("//evil.test") && !safe_href("javascript:alert(1)"));
        assert!(safe_href("/about") && safe_href("https://acme.test") && safe_href("mailto:a@acme.test"));
    }

    #[test]
    fn form_addresses_are_plain() {
        assert!(form_ok("contact-us") && form_ok("a1"));
        assert!(!form_ok("") && !form_ok("Contact") && !form_ok("a/b") && !form_ok("a?x=1") && !form_ok(&"a".repeat(81)) && !form_ok("a\"b"));
    }

    #[test]
    fn crawlers_are_told_apart() {
        assert!(crawler("Mozilla/5.0 (compatible; Googlebot/2.1)") && crawler("facebookexternalhit Preview") && crawler("SomeSpider"));
        assert!(!crawler("Mozilla/5.0 (Macintosh) Safari/605.1.15") && !crawler(""));
    }
}
