//! The workspace's public website (FERRUM-ECOSYSTEM-006): pages and blog posts written in
//! Markdown, a menu, a home page, and the site served on its own `site:website` host, rendered on
//! the server so search engines and link previews read it (Ferrum docs/plan/apps-ui.md §7.3).
//!
//! * `pages`: addresses made from titles, kept off the site's own paths; blocks, the menu's one
//!   level, languages and search-engine fields checked; publishing and taking back; pages made
//!   from templates.
//! * `redirects`: old addresses leading on, one step only, to another site only when an
//!   administrator says so; made when a published page's address changes.
//! * `site`: the public pages with their blocks, brand and languages, the blog, the sitemap and
//!   robots.txt, redirects served, views counted; safe Markdown: raw HTML never passes, links are
//!   the web's, mail or the site's own, and images are https:// only.
//! * `reports`: the addresses most viewed.
//!
//! This app's own design; no code was copied (docs/PROVENANCE.md).

mod pages;
mod redirects;
mod reports;
mod site;

use ferrum_domain::reports::ReportSource;
use ferrum_kernel::{HookRegistry, Kernel, Result};
use std::sync::Arc;

include!(concat!(env!("OUT_DIR"), "/packages.rs"));
include!(concat!(env!("OUT_DIR"), "/ui.rs"));

/// This app's manifest (`app.yaml`).
pub const MANIFEST: &str = include_str!("../app.yaml");

/// This app's metadata packages, embedded at build time.
pub fn packages() -> std::result::Result<Vec<ferrum_meta::Package>, ferrum_meta::MetaError> {
    ferrum_meta::catalog::load_embedded(EMBEDDED)
}

/// Registers this app's document hooks and its public site.
pub fn register(hooks: &mut HookRegistry) {
    hooks.register("WebPage", Arc::new(pages::PageHook));
    hooks.register("WebRedirect", Arc::new(redirects::RedirectHook));
    hooks.register("WebsiteSettings", Arc::new(pages::SettingsHook));
    hooks.add_site("website", Arc::new(site::WebsiteSite));
}

/// This app's reports: the addresses most viewed.
pub fn reports() -> Vec<Arc<dyn ReportSource>> {
    vec![Arc::new(reports::WebsiteReports)]
}

/// Upkeep: page views are kept 400 days.
pub async fn maintenance(k: &Kernel, tenant: uuid::Uuid) -> Result<()> {
    let mut tx = k.db.tenant_tx(tenant).await?;
    sqlx::query("delete from website_view where tenant_id = $1 and day < current_date - 400").bind(tenant).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_interface_is_a_release_the_platform_takes() {
        let m = ferrum_apps::AppManifest::parse(MANIFEST).expect("app.yaml");
        assert_eq!(m.routes(), vec![("website".to_string(), vec![])]);
        let files = UI.iter().map(|(p, b)| (p.to_string(), b.to_vec())).collect();
        let ui = ferrum_apps::ui::prepare("website", files).expect("ui/dist is an interface release");
        assert!(UI.iter().any(|(p, _)| *p == "index.html") && !UI.iter().any(|(p, _)| p.starts_with('.')), "{}", ui.digest);
    }

    /// People, agents and outside callers use the same operations: each says what it changes, what it returns and
    /// what "done" means (`scripts/agent_parity.py` in the platform).
    #[test]
    fn every_action_has_a_contract() {
        let mut seen = 0;
        for p in packages().expect("packages") {
            for e in &p.entities {
                for a in &e.actions {
                    let c = a.contract.as_ref().unwrap_or_else(|| panic!("{}.{} has no contract", e.name, a.name));
                    let v = serde_json::to_value(c).expect("contract");
                    for key in ["effect", "returns", "completion"] {
                        assert!(!v[key].is_null(), "{}.{}: contract without {key}: {v}", e.name, a.name);
                    }
                    seen += 1;
                }
            }
        }
        assert!(seen >= 2, "publish and unpublish at least");
    }

    #[test]
    fn addresses_from_titles() {
        use pages::slugify;
        assert_eq!(slugify("About Us"), "about-us");
        assert_eq!(slugify("  Hello, World!  2026 "), "hello-world-2026");
        assert_eq!(slugify("--Café & Bar--"), "caf-bar");
        assert_eq!(slugify("नमस्ते"), "");
        assert_eq!(slugify(&"a".repeat(200)).len(), 120);
    }

    #[test]
    fn markdown_without_scripts() {
        use site::markdown;
        let html = markdown(
            "# Hi\n\n<script>alert(1)</script>\n\nText <b onclick=\"x()\">bold</b> [ok](https://acme.test) [bad](javascript:alert(1)) [mail](mailto:a@acme.test) [here](/about) [far](//evil.test)\n\n![a](https://acme.test/a.png) ![b](http://acme.test/b.png) ![c](data:image/png;base64,AA)",
        );
        assert!(!html.contains("<script") && !html.contains("onclick") && !html.contains("javascript:") && !html.contains("//evil.test"), "{html}");
        assert!(
            html.contains(r#"<a href="https://acme.test">ok</a>"#)
                && html.contains(r#"<a href="mailto:a@acme.test">"#)
                && html.contains(r#"<a href="/about">"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<img src="https://acme.test/a.png" alt="a""#) && !html.contains("http://acme.test/b.png") && !html.contains("data:image"),
            "{html}"
        );
        assert!(html.contains("bad") && html.contains("far"), "the text of a dropped link stays: {html}");
    }
}
