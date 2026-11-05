//! The workspace's public website (FERRUM-ECOSYSTEM-006): pages and blog posts written in
//! Markdown, a menu, a home page, and the site served on its own `site:website` host, rendered on
//! the server so search engines and link previews read it (Ferrum docs/plan/apps-ui.md §7.3).
//!
//! * `pages`: addresses made from titles, kept off the site's own paths; publishing and taking
//!   back.
//! * `site`: the public pages, the blog, the sitemap and robots.txt, and safe Markdown: raw HTML
//!   never passes, links are the web's, mail or the site's own, and images are https:// only.
//!
//! This app's own design; no code was copied (docs/PROVENANCE.md).

mod pages;
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

/// Registers this app's document hook and its public site.
pub fn register(hooks: &mut HookRegistry) {
    hooks.register("WebPage", Arc::new(pages::PageHook));
    hooks.add_site("website", Arc::new(site::WebsiteSite));
}

/// This app's reports: none of its own yet.
pub fn reports() -> Vec<Arc<dyn ReportSource>> {
    vec![]
}

/// Upkeep: nothing yet.
pub async fn maintenance(_k: &Kernel) -> Result<()> {
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
