//! Pages and posts: an address (`slug`) made from the title when none is given, in lower-case
//! letters, digits and dashes, never one of the site's own paths; published with the time, or
//! taken back to a draft.

use async_trait::async_trait;
use chrono::Utc;
use ferrum_domain::util::{id, s, sys};
use ferrum_kernel::{Doc, DocHook, Error, HookCtx, Result};
use serde_json::{Value, json};

/// Paths the site keeps for itself (`/sitemap.xml` and `/robots.txt` have dots no address has).
pub(crate) const RESERVED: &[&str] = &["blog", "assets"];

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

    async fn validate(&self, _hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        match s(doc, "slug") {
            None => Err(Error::field("slug", "needs letters or digits: give the page an address")),
            Some(slug) if RESERVED.contains(&slug) => Err(Error::field("slug", format!("/{slug} is the site's own; give the page another address"))),
            Some(_) => Ok(()),
        }
    }

    async fn action(&self, hc: &mut HookCtx<'_>, name: &str, doc: &Doc, _params: &Value) -> Result<Option<Value>> {
        let page = id(doc, "id").ok_or_else(|| Error::internal("a page without an id"))?;
        let mut v = Doc::new();
        match name {
            "publish" => {
                v.insert("status".into(), json!("Published"));
                v.insert("published_at".into(), json!(Utc::now().to_rfc3339()));
            }
            "unpublish" => {
                v.insert("status".into(), json!("Draft"));
            }
            _ => return Ok(None),
        }
        let status = v["status"].clone();
        hc.kernel.set_values_tx(&sys(hc.ctx), hc.tx, "WebPage", page, v).await?;
        Ok(Some(json!({"status": status, "slug": s(doc, "slug")})))
    }
}
