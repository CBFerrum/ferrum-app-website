//! The public site. On a `site:website` host the platform hands every path to `/_site/{path}`
//! with the site's own origin (`x-ferrum-site-origin`, never the caller's); the pages are rendered
//! here, on the server, with no script:
//!
//! * `/`: the home page the settings name, else the latest posts;
//! * `/blog`: the posts, newest first; `/{slug}`: a published page or post;
//! * `/sitemap.xml`, `/robots.txt`, and `/assets/site.css`.
//!
//! Drafts are never served. Text is escaped; Markdown goes through [`markdown`].

use crate::pages::slugify;
use ferrum_domain::util::{id, s};
use ferrum_kernel::{Ctx, Doc, Error, Filter, Kernel, Result, Site, SiteRequest, SiteResponse, Tx};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde_json::Value;
use uuid::Uuid;

const CSS: &str = include_str!("../assets/site.css");
/// Posts on the blog and the home page at most.
const POSTS: u32 = 50;

pub struct WebsiteSite;

#[async_trait::async_trait]
impl Site for WebsiteSite {
    async fn handle(&self, k: &Kernel, req: SiteRequest) -> Result<SiteResponse> {
        let path = req.path.trim_start_matches('/');
        let Some(rest) = path.strip_prefix("_site").filter(|r| r.is_empty() || r.starts_with('/')) else {
            return Err(Error::NotFound(format!("website endpoint {} {}", req.method, req.path)));
        };
        if req.method != "GET" {
            return Err(Error::NotFound("the site is read only".into()));
        }
        let origin = req.headers.get("x-ferrum-site-origin").and_then(Value::as_str).unwrap_or_default().trim_end_matches('/').to_string();
        let ctx = Ctx::system(req.tenant, k.registry.schema(req.tenant).await?);
        let mut tx = k.db.tenant_tx(req.tenant).await?;
        let out = page(k, &ctx, &mut tx, rest.trim_matches('/'), &origin).await;
        tx.commit().await?;
        out
    }
}

/// The site's name, words and menu, from its settings and its published pages.
struct Info {
    title: String,
    tagline: String,
    blog: String,
    footer: String,
    home: Option<Uuid>,
    menu: Vec<(String, String)>,
}

async fn info(k: &Kernel, ctx: &Ctx, tx: &mut Tx) -> Result<Info> {
    let settings = k.find_tx(ctx, tx, "WebsiteSettings", &[], None, 1).await?.into_iter().next().unwrap_or_default();
    let text = |f: &str, default: &str| s(&settings, f).unwrap_or(default).to_string();
    let mut menu: Vec<Doc> =
        k.find_tx(ctx, tx, "WebPage", &[Filter::eq("status", "Published"), Filter::eq("in_menu", true)], Some("menu_order asc"), 30).await?;
    menu.sort_by_key(|p| (p.get("menu_order").and_then(Value::as_i64).unwrap_or(100), s(p, "title").unwrap_or_default().to_lowercase()));
    Ok(Info {
        title: text("site_title", "Website"),
        tagline: text("tagline", ""),
        blog: text("blog_title", "Blog"),
        footer: text("footer", ""),
        home: id(&settings, "home_page"),
        menu: menu.iter().filter_map(|p| Some((s(p, "slug")?.to_string(), s(p, "title")?.to_string()))).collect(),
    })
}

async fn page(k: &Kernel, ctx: &Ctx, tx: &mut Tx, path: &str, origin: &str) -> Result<SiteResponse> {
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
            for p in &pages {
                let day = s(p, "published_at").map(|t| t.chars().take(10).collect::<String>()).unwrap_or_default();
                xml.push_str(&format!("<url><loc>{}/{}</loc><lastmod>{}</lastmod></url>", esc(origin), esc(s(p, "slug").unwrap_or_default()), esc(&day)));
            }
            xml.push_str("</urlset>");
            return Ok(SiteResponse::text(200, "application/xml; charset=utf-8", xml));
        }
        _ => {}
    }
    let site = info(k, ctx, tx).await?;
    let found = |slug: &str| Filter::eq("slug", slugify(slug));
    let doc = match path {
        "" => match site.home {
            Some(home) => {
                k.find_tx(ctx, tx, "WebPage", &[Filter::eq("id", home.to_string()), Filter::eq("status", "Published")], None, 1).await?.into_iter().next()
            }
            None => None,
        },
        "blog" => None,
        slug if slugify(slug) == slug => {
            let hit = k.find_tx(ctx, tx, "WebPage", &[found(slug), Filter::eq("status", "Published")], None, 1).await?.into_iter().next();
            Some(hit.ok_or_else(|| Error::NotFound("page".into()))?)
        }
        _ => return Err(Error::NotFound("page".into())),
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
            let main = format!("<article><h1>{}</h1>{when}{}</article>", esc(title), markdown(s(&p, "body").unwrap_or_default()));
            layout(&site, origin, &format!("/{}", s(&p, "slug").unwrap_or_default()), title, s(&p, "summary").unwrap_or_default(), &main)
        }
        None => {
            let posts =
                k.find_tx(ctx, tx, "WebPage", &[Filter::eq("status", "Published"), Filter::eq("kind", "Post")], Some("published_at desc"), POSTS).await?;
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
            let list = if items.is_empty() { r#"<p class="muted">Nothing here yet.</p>"#.to_string() } else { format!(r#"<ul class="posts">{items}</ul>"#) };
            let (path, heading) = if path == "blog" { ("/blog", site.blog.clone()) } else { ("/", site.title.clone()) };
            let tagline = site.tagline.clone();
            layout(&site, origin, path, &heading, &tagline, &format!("<h1>{}</h1>{list}", esc(&heading)))
        }
    };
    Ok(SiteResponse::text(200, "text/html; charset=utf-8", html))
}

fn layout(site: &Info, origin: &str, path: &str, title: &str, description: &str, main: &str) -> String {
    let full = if title.is_empty() || title == site.title { esc(&site.title) } else { format!("{} · {}", esc(title), esc(&site.title)) };
    let description = if description.is_empty() { String::new() } else { format!(r#"<meta name="description" content="{}">"#, esc(description)) };
    let canonical = if origin.is_empty() { String::new() } else { format!(r#"<link rel="canonical" href="{}{}">"#, esc(origin), esc(path)) };
    let menu: String = site.menu.iter().map(|(slug, t)| format!(r#"<li><a href="/{}">{}</a></li>"#, esc(slug), esc(t))).collect();
    let tagline = if site.tagline.is_empty() { String::new() } else { format!(r#"<p class="tagline">{}</p>"#, esc(&site.tagline)) };
    let footer = if site.footer.is_empty() { format!("© {}", esc(&site.title)) } else { esc(&site.footer) };
    format!(
        r##"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>{full}</title>{description}{canonical}<link rel="stylesheet" href="/assets/site.css?v={v}"></head><body><a class="skip" href="#main">Skip to content</a><header class="top"><a class="brand" href="/">{brand}</a>{tagline}<nav aria-label="Site"><ul>{menu}<li><a href="/blog">{blog}</a></li></ul></nav></header><main id="main">{main}</main><footer>{footer}</footer></body></html>"##,
        v = CSS.len(),
        brand = esc(&site.title),
        blog = esc(&site.blog),
    )
}

/// Text for HTML, attributes included.
fn esc(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}

/// Where a link may go: the web, mail, or this site's own paths (not another site's by `//`).
fn safe_href(url: &str) -> bool {
    let plain = !url.chars().any(|c| c.is_whitespace() || c.is_control());
    plain
        && (url.starts_with("https://")
            || url.starts_with("http://")
            || url.starts_with("mailto:")
            || url.starts_with('#')
            || (url.starts_with('/') && !url.starts_with("//")))
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
