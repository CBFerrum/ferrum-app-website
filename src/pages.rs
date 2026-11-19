//! Pages and posts: an address (`slug`) made from the title when none is given, in lower-case
//! letters, digits and dashes, never one of the site's own paths; their blocks, place in the menu
//! (one level), language and search-engine fields checked; published with the time, or taken back to
//! a draft; a page made from a template. A published page whose address changes leaves a redirect
//! from the old one (`crate::redirects`).

use crate::redirects;
use crate::site::{form_ok, image_ok, safe_href, workspace};
use async_trait::async_trait;
use chrono::Utc;
use ferrum_domain::util::{b, id, rows, s, sys};
use ferrum_kernel::capability::Invocation;
use ferrum_kernel::{Ctx, Doc, DocHook, Error, Filter, HookCtx, Result};
use serde_json::{Value, json};
use uuid::Uuid;

/// Paths the site keeps for itself (`/sitemap.xml` and `/robots.txt` have dots no address has).
pub(crate) const RESERVED: &[&str] = &["blog", "assets"];
/// Blocks on one page at most.
pub(crate) const BLOCKS_MAX: usize = 60;
/// A block's own fields: what a page made from a template copies.
const BLOCK_FIELDS: &[&str] = &["kind", "heading", "text", "image", "image_alt", "button_label", "button_url", "form", "posts"];

/// An address from any text: lower-case ASCII letters and digits, other runs as one dash, at most
/// 120 characters.
pub(crate) fn slugify(text: &str) -> String {
    let mut out = String::new();
    for c in text.trim().to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let cut: String = out.chars().take(120).collect();
    cut.trim_end_matches('-').to_string()
}

/// Holds `what` for the workspace until the transaction ends: checks that read other pages (the menu's one level) or
/// other redirects (no chains) stay true when two saves race.
pub(crate) async fn lock(hc: &mut HookCtx<'_>, what: &str) -> Result<()> {
    sqlx::query("select pg_advisory_xact_lock(hashtextextended($1, 0))").bind(format!("website-{what}:{}", hc.ctx.tenant)).execute(&mut **hc.tx).await?;
    Ok(())
}

pub struct PageHook;

#[async_trait]
impl DocHook for PageHook {
    fn name(&self) -> &'static str {
        "website.page"
    }

    fn fills(&self) -> &'static [&'static str] {
        &["slug"]
    }

    async fn before_validate(&self, _hc: &mut HookCtx<'_>, doc: &mut Doc) -> Result<()> {
        let from = s(doc, "slug").or_else(|| s(doc, "title")).unwrap_or_default().to_string();
        let slug = slugify(&from);
        doc.insert("slug".into(), if slug.is_empty() { Value::Null } else { json!(slug) });
        Ok(())
    }

    async fn validate(&self, hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        match s(doc, "slug") {
            None => return Err(Error::field("slug", "needs letters or digits: give the page an address")),
            Some(slug) if RESERVED.contains(&slug) => return Err(Error::field("slug", format!("/{slug} is the site's own; give the page another address"))),
            Some(_) => {}
        }
        let ws = workspace(hc.tx, hc.ctx.tenant).await?;
        let blocks = rows(doc, "blocks");
        if blocks.len() > BLOCKS_MAX {
            return Err(Error::field("blocks", format!("a page has at most {BLOCKS_MAX} blocks")));
        }
        for (n, bl) in blocks.iter().enumerate() {
            let n = n + 1;
            let kind = s(bl, "kind").unwrap_or_default();
            match s(bl, "image") {
                Some(u) if !image_ok(u, &ws.slug) => {
                    return Err(Error::field(
                        "blocks",
                        format!("block {n}: a picture is a public file (/files/public/…), a DAM web address or an https:// address"),
                    ));
                }
                None if kind == "Image" => return Err(Error::field("blocks", format!("block {n}: an Image block needs its picture"))),
                _ => {}
            }
            if s(bl, "button_url").is_some_and(|u| !safe_href(u)) {
                return Err(Error::field("blocks", format!("block {n}: a button goes to https://, mailto: or a path of the site")));
            }
            match s(bl, "form") {
                Some(f) if !form_ok(f) => {
                    return Err(Error::field("blocks", format!("block {n}: a form's address name is lower-case letters, digits and dashes")));
                }
                None if kind == "Form" => return Err(Error::field("blocks", format!("block {n}: a Form block needs the form's address name"))),
                _ => {}
            }
        }
        if s(doc, "og_image").is_some_and(|u| !image_ok(u, &ws.slug)) {
            return Err(Error::field("og_image", "a public file (/files/public/…), a DAM web address or an https:// address"));
        }
        if s(doc, "canonical_url").is_some_and(|u| !(u.starts_with("https://") && safe_href(u))) {
            return Err(Error::field("canonical_url", "an https:// address"));
        }
        let language = s(doc, "language");
        if language.is_some_and(|l| !ferrum_kernel::tenant::valid_language(l)) {
            return Err(Error::field("language", "a language tag such as en, hi or pt-BR"));
        }
        let this = id(doc, "id");
        let template = b(doc, "is_template");
        if template && b(doc, "in_menu") {
            return Err(Error::field("in_menu", "a template is never served, so it is not in the menu"));
        }
        let sys = sys(hc.ctx);
        if let Some(of) = id(doc, "translation_of") {
            let Some(language) = language else { return Err(Error::field("language", "a translation says its language")) };
            if Some(of) == this {
                return Err(Error::field("translation_of", "a page does not translate itself"));
            }
            let original = hc.kernel.get_tx(&sys, hc.tx, "WebPage", of, false).await?;
            if id(&original, "translation_of").is_some() {
                return Err(Error::field("translation_of", "name the page it translates, not another translation of it"));
            }
            if s(&original, "language") == Some(language) {
                return Err(Error::field("language", "the page it translates is in this language already"));
            }
            if template || b(&original, "is_template") {
                return Err(Error::field("translation_of", "templates are not translated"));
            }
        }
        if let Some(parent) = id(doc, "menu_parent") {
            lock(hc, "menu").await?;
            if Some(parent) == this {
                return Err(Error::field("menu_parent", "a page is not under itself"));
            }
            if template {
                return Err(Error::field("menu_parent", "a template is not in the menu"));
            }
            let up = hc.kernel.get_tx(&sys, hc.tx, "WebPage", parent, false).await?;
            if id(&up, "menu_parent").is_some() {
                return Err(Error::field("menu_parent", "the menu has one level: choose a page at its top"));
            }
            if !b(&up, "in_menu") || b(&up, "is_template") {
                return Err(Error::field("menu_parent", "choose a page that is in the menu"));
            }
            if let Some(this) = this
                && !hc.kernel.find_tx(&sys, hc.tx, "WebPage", &[Filter::eq("menu_parent", this.to_string())], None, 1).await?.is_empty()
            {
                return Err(Error::field("menu_parent", "pages are under this one: the menu has one level"));
            }
        }
        // A published page's form frames show a published form, never a frame saying there is none.
        if s(doc, "status") == Some("Published") {
            forms_published(hc, doc).await?;
        }
        Ok(())
    }

    async fn after_save(&self, hc: &mut HookCtx<'_>, doc: &Doc, old: Option<&Doc>) -> Result<()> {
        let (Some(slug), true) = (s(doc, "slug"), s(doc, "status") == Some("Published")) else { return Ok(()) };
        match old.filter(|o| s(o, "status") == Some("Published")).and_then(|o| s(o, "slug")) {
            Some(before) if before != slug => redirects::moved(hc, &format!("/{before}"), &format!("/{slug}")).await,
            _ => redirects::page_is_at(hc, &format!("/{slug}")).await,
        }
    }

    async fn action(&self, hc: &mut HookCtx<'_>, name: &str, doc: &Doc, params: &Value) -> Result<Option<Value>> {
        let page = id(doc, "id").ok_or_else(|| Error::internal("a page without an id"))?;
        let mut v = Doc::new();
        match name {
            "publish" => {
                if b(doc, "is_template") {
                    return Err(Error::Invalid("a template is never published: make a page from it".into()));
                }
                forms_published(hc, doc).await?;
                if let Some(slug) = s(doc, "slug") {
                    redirects::page_is_at(hc, &format!("/{slug}")).await?;
                }
                v.insert("status".into(), json!("Published"));
                v.insert("published_at".into(), json!(Utc::now().to_rfc3339()));
            }
            "unpublish" => {
                v.insert("status".into(), json!("Draft"));
            }
            "use_template" => return use_template(hc, doc, params).await.map(Some),
            _ => return Ok(None),
        }
        let status = v["status"].clone();
        hc.kernel.set_values_tx(&sys(hc.ctx), hc.tx, "WebPage", page, v).await?;
        Ok(Some(json!({"status": status, "slug": s(doc, "slug")})))
    }
}

/// Every Form block of `doc` names a published public form of the workspace. Asked of Forms through the gate, as the
/// system: whether a form is published at an address is public (anyone can open it), and the person writing the page
/// need not read Forms' records. The answer is yes or no, nothing of the form.
async fn forms_published(hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
    let wanted: Vec<String> =
        rows(doc, "blocks").iter().filter(|bl| s(bl, "kind") == Some("Form")).filter_map(|bl| s(bl, "form").map(str::to_string)).collect();
    if wanted.is_empty() {
        return Ok(());
    }
    if hc.ctx.schema.entity("Form").is_none() {
        return Err(Error::field("blocks", "Forms is not installed: take the Form block out"));
    }
    let sys = Ctx::system(hc.ctx.tenant, hc.ctx.schema.clone());
    for form in wanted {
        let ask = Invocation::new("forms.form.list", json!({"filters": {"slug": form, "status": "Published", "access": "Public"}, "limit": 1}));
        let found = hc.kernel.invoke_tx(&sys, hc.tx, &ask).await?;
        if found.result.as_array().is_none_or(|r| r.is_empty()) {
            return Err(Error::field("blocks", format!("no form with the address {form} is published: publish it in Forms, or correct the address")));
        }
    }
    Ok(())
}

/// `use_template {title}`: a draft page with the template's text and blocks and the title given, made with the
/// caller's own right to make pages.
async fn use_template(hc: &mut HookCtx<'_>, doc: &Doc, params: &Value) -> Result<Value> {
    if !b(doc, "is_template") {
        return Err(Error::Invalid("only a template makes pages".into()));
    }
    let title =
        params.get("title").and_then(Value::as_str).map(str::trim).filter(|t| !t.is_empty()).ok_or_else(|| Error::field("title", "give the page a title"))?;
    let blocks: Vec<Value> = rows(doc, "blocks")
        .into_iter()
        .map(|bl| Value::Object(bl.into_iter().filter(|(k, v)| BLOCK_FIELDS.contains(&k.as_str()) && !v.is_null()).collect()))
        .collect();
    let mut page = Doc::new();
    page.insert("title".into(), json!(title));
    for f in ["kind", "summary", "body", "language"] {
        if let Some(v) = doc.get(f).filter(|v| !v.is_null()) {
            page.insert(f.into(), v.clone());
        }
    }
    page.insert("blocks".into(), Value::Array(blocks));
    let made = hc.kernel.insert_tx(hc.ctx, hc.tx, "WebPage", page).await?;
    let made: Option<Uuid> = id(&made, "id");
    Ok(json!({"page": made}))
}

/// Links of the settings' menu go where a link of a page may go.
pub struct SettingsHook;

/// Links in the menu at most.
const LINKS_MAX: usize = 20;

#[async_trait]
impl DocHook for SettingsHook {
    fn name(&self) -> &'static str {
        "website.settings"
    }

    async fn validate(&self, _hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        let links = rows(doc, "menu_links");
        if links.len() > LINKS_MAX {
            return Err(Error::field("menu_links", format!("the menu takes at most {LINKS_MAX} links")));
        }
        for (n, l) in links.iter().enumerate() {
            if !s(l, "url").is_some_and(safe_href) {
                return Err(Error::field("menu_links", format!("link {}: an address is https://, mailto: or a path of the site", n + 1)));
            }
        }
        Ok(())
    }
}
