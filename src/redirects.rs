//! Redirects: an old address of the site that leads to a new one, one step only. Kept so that no
//! chain can form (a target is never another redirect's start, a start never another's target), so
//! there is no loop to follow; a redirect to another site only when an administrator named it; and
//! made by the site itself when a published page's address changes.

use crate::pages::lock;
use crate::site::{PATH_MAX, safe_href};
use async_trait::async_trait;
use ferrum_domain::util::{id, s, sys};
use ferrum_kernel::{Doc, DocHook, Error, Filter, HookCtx, Result};
use serde_json::{Value, json};

/// A path of the site as a redirect starts from or goes to: `/`, then no `/` at once, no backslash, whitespace or
/// control character, not one of the platform's own (`/_…`).
pub(crate) fn path_ok(p: &str) -> bool {
    p.len() <= PATH_MAX
        && p.starts_with('/')
        && !p.starts_with("//")
        && !p.starts_with("/_")
        && !p.chars().any(|c| c.is_whitespace() || c.is_control() || c == '\\')
}

/// Another site's address (`https://` only).
fn elsewhere(to: &str) -> bool {
    to.starts_with("https://") && safe_href(to)
}

/// Where a redirect may send a visitor (checked again when served).
pub(crate) fn target_ok(to: &str) -> bool {
    path_ok(to) || elsewhere(to)
}

/// A path as kept: with its `/`, without one at the end.
fn normal(p: &str) -> String {
    let p = p.trim();
    let p = if p.len() > 1 { p.trim_end_matches('/') } else { p };
    if p.starts_with('/') { p.to_string() } else { format!("/{p}") }
}

pub struct RedirectHook;

#[async_trait]
impl DocHook for RedirectHook {
    fn name(&self) -> &'static str {
        "website.redirect"
    }

    fn fills(&self) -> &'static [&'static str] {
        &["from_path", "to"]
    }

    async fn before_validate(&self, _hc: &mut HookCtx<'_>, doc: &mut Doc) -> Result<()> {
        if let Some(from) = s(doc, "from_path").map(normal) {
            doc.insert("from_path".into(), json!(from));
        }
        if let Some(to) = s(doc, "to").map(str::trim).map(|t| if t.starts_with('/') { normal(t) } else { t.to_string() }) {
            doc.insert("to".into(), json!(to));
        }
        Ok(())
    }

    async fn validate(&self, hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        let from = s(doc, "from_path").unwrap_or_default();
        let to = s(doc, "to").unwrap_or_default();
        if from == "/" || !path_ok(from) {
            return Err(Error::field("from_path", "a path of the site such as /old-page"));
        }
        if !target_ok(to) {
            return Err(Error::field("to", "a path of the site (/new-page) or an https:// address"));
        }
        if to == from {
            return Err(Error::field("to", "a redirect leads somewhere else"));
        }
        // One step only: no other redirect starts where this one ends, or ends where this one starts. Held while two
        // saves race.
        lock(hc, "redirect").await?;
        let this = id(doc, "id");
        let sys = sys(hc.ctx);
        let other = |found: Vec<Doc>| found.into_iter().any(|d| id(&d, "id") != this);
        if other(hc.kernel.find_tx(&sys, hc.tx, "WebRedirect", &[Filter::eq("from_path", to)], None, 2).await?) {
            return Err(Error::field("to", format!("{to} leads on elsewhere: point this redirect where that one goes")));
        }
        if other(hc.kernel.find_tx(&sys, hc.tx, "WebRedirect", &[Filter::eq("to", from)], None, 2).await?) {
            return Err(Error::field("from_path", format!("another redirect leads to {from}: point it where this one goes")));
        }
        Ok(())
    }

    async fn validate_change(&self, hc: &mut HookCtx<'_>, old: Option<&Doc>, doc: &Doc) -> Result<()> {
        // Sending visitors to another site is an administrator's decision.
        let to = s(doc, "to").unwrap_or_default();
        let changed = old.and_then(|o| s(o, "to")) != Some(to);
        if changed && elsewhere(to) && !hc.ctx.is_admin {
            return Err(Error::field("to", "only an administrator sends visitors to another site; give a path of this site"));
        }
        Ok(())
    }
}

/// A published page moved from `from` to `to`: visitors of the old address are led to the new one, in one step.
pub(crate) async fn moved(hc: &mut HookCtx<'_>, from: &str, to: &str) -> Result<()> {
    lock(hc, "redirect").await?;
    let sys = sys(hc.ctx);
    page_is_at(hc, to).await?;
    // Those that led to the old address lead to the new one: still one step.
    for r in hc.kernel.find_tx(&sys, hc.tx, "WebRedirect", &[Filter::eq("to", from)], None, 1000).await? {
        if let Some(rid) = id(&r, "id") {
            let mut v = Doc::new();
            v.insert("to".into(), json!(to));
            hc.kernel.update_tx(&sys, hc.tx, "WebRedirect", rid, v, None).await?;
        }
    }
    let mine: Doc = [("from_path".to_string(), json!(from)), ("to".into(), json!(to)), ("permanent".into(), Value::Bool(true))].into_iter().collect();
    match hc.kernel.find_tx(&sys, hc.tx, "WebRedirect", &[Filter::eq("from_path", from)], None, 1).await?.first().and_then(|r| id(r, "id")) {
        Some(rid) => hc.kernel.update_tx(&sys, hc.tx, "WebRedirect", rid, mine, None).await.map(|_| ()),
        None => hc.kernel.insert_tx(&sys, hc.tx, "WebRedirect", mine).await.map(|_| ()),
    }
}

/// A published page is at `path`: a redirect from there would never be served, so it goes.
pub(crate) async fn page_is_at(hc: &mut HookCtx<'_>, path: &str) -> Result<()> {
    let sys = sys(hc.ctx);
    for r in hc.kernel.find_tx(&sys, hc.tx, "WebRedirect", &[Filter::eq("from_path", path)], None, 1).await? {
        if let Some(rid) = id(&r, "id") {
            hc.kernel.delete_tx(&sys, hc.tx, "WebRedirect", rid, None).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_stay_on_the_site() {
        assert!(path_ok("/old-page") && path_ok("/a/b") && path_ok("/"));
        for bad in ["old", "//evil.test", "/\\evil.test", "/_shell/x", "/a b", "/a\tb", &format!("/{}", "a".repeat(300))] {
            assert!(!path_ok(bad), "{bad}");
        }
        assert!(target_ok("https://shop.acme.test/x") && !target_ok("http://acme.test") && !target_ok("javascript:alert(1)"));
        assert_eq!((normal("old/"), normal("/old//"), normal("/")), ("/old".into(), "/old".into(), "/".into()));
    }
}
