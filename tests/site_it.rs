//! The public site's pages against a real PostgreSQL (`FERRUM_WEBSITE_SITE_TEST_DATABASE_URL`, default
//! `postgres://ferrum:ferrum@localhost:5432/ferrum_website_site_test`; reset first), asked for as the platform hands a
//! `site:website` host's requests to the package: blocks escaped, form frames only for published forms of the
//! workspace, the menu's one level and its links, search-engine fields, one-step redirects, templates, the brand,
//! languages, and view counts a flood cannot grow. Each check runs on a workspace of its own, in turn.

use ferrum_kernel::tenant::NewTenant;
use ferrum_kernel::{Ctx, Db, DbConfig, Doc, Error, HookRegistry, Kernel, KernelSettings, Site, SiteRequest};
use ferrum_meta::Catalog;
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Forms' `Form` as far as Website reads it (through the gate's `forms.form.list`): its address name, state and access.
const FORMS_PACKAGE: &str = "package: forms\nversion: 0.1.0\nlabel: Forms\ndepends: [core]\nmodules:\n  - {name: forms, label: Forms, items: [Form]}\n";
const FORMS_ENTITIES: &str = "- name: Form\n  label: Form\n  kind: master\n  module: forms\n  title_field: title\n  fields:\n    - {name: title, type: data, required: true}\n    - {name: slug, type: data, unique: true}\n    - {name: status, type: select, options: [Draft, Published, Closed], default: Draft}\n    - {name: access, type: select, options: [Public, Internal], default: Public}\n";

fn obj(v: Value) -> Doc {
    v.as_object().cloned().unwrap()
}

fn uid(v: &Value) -> Uuid {
    v.as_str().unwrap_or_else(|| panic!("not an id: {v}")).parse().unwrap()
}

struct World {
    k: Arc<Kernel>,
    site: Arc<dyn Site>,
}

/// A workspace with Website (and Forms' stand-in when `forms`): its administrator and a Site Editor.
struct Ws {
    tenant: Uuid,
    admin: Ctx,
    editor: Ctx,
}

impl World {
    async fn workspace(&self, slug: &str, forms: bool) -> Ws {
        let k = &self.k;
        let mut packages = vec!["website".to_string()];
        if forms {
            packages.push("forms".into());
        }
        let tenant = k
            .create_tenant(NewTenant {
                slug: slug.into(),
                name: format!("Acme {slug}"),
                admin_email: format!("admin@{slug}.test"),
                admin_password: "Correct-Horse-9".into(),
                admin_name: None,
                packages,
                settings: Some(json!({"timezone": "UTC"})),
            })
            .await
            .expect("tenant");
        let s = k.login(slug, &format!("admin@{slug}.test"), "Correct-Horse-9").await.expect("login");
        let admin = k.load_ctx(tenant, s.user, Some(s.token_version), "it".into()).await.expect("ctx");
        let email = format!("web@{slug}.test");
        let id = k.create_user(&admin, &email, &email, Some("Correct-Horse-9"), &["Site Editor".to_string()], false).await.expect("user");
        let s = k.login(slug, &email, "Correct-Horse-9").await.expect("login");
        let editor = k.load_ctx(tenant, id, Some(s.token_version), "it".into()).await.expect("ctx");
        Ws { tenant, admin, editor }
    }

    /// A public page as the platform asks for it on `host`: (status, headers, body), or the error.
    async fn get_as(&self, tenant: Uuid, path: &str, agent: &str) -> Result<(u16, Vec<(String, String)>, String), Error> {
        let mut headers = Map::new();
        headers.insert("x-ferrum-site-origin".into(), json!("https://www.acme.test"));
        headers.insert("user-agent".into(), json!(agent));
        let req = SiteRequest {
            tenant,
            user: None,
            method: "GET".into(),
            path: format!("/_site{path}"),
            query: Map::new(),
            headers,
            body: vec![],
            client_ip: "203.0.113.7".into(),
        };
        let r = self.site.handle(&self.k, req).await?;
        Ok((r.status, r.headers, String::from_utf8(r.body).unwrap()))
    }

    async fn get(&self, tenant: Uuid, path: &str) -> Result<String, Error> {
        self.get_as(tenant, path, "Mozilla/5.0 (Macintosh) Safari/605.1.15").await.map(|(_, _, b)| b)
    }

    async fn preview(&self, who: &Ctx, page: &Value) -> String {
        let req = SiteRequest {
            tenant: who.tenant,
            user: Some(who.clone()),
            method: "GET".into(),
            path: format!("/preview/{}", page.as_str().unwrap()),
            query: Map::new(),
            headers: Map::new(),
            body: vec![],
            client_ip: "203.0.113.7".into(),
        };
        let r = self.site.handle(&self.k, req).await.expect("preview");
        serde_json::from_slice::<Value>(&r.body).unwrap()["html"].as_str().unwrap().to_string()
    }

    async fn page(&self, who: &Ctx, v: Value) -> Doc {
        self.k.insert(who, "WebPage", obj(v)).await.expect("page")
    }

    async fn publish(&self, who: &Ctx, page: &Doc) {
        self.k.run_action(who, "WebPage", uid(&page["id"]), "publish", Value::Null).await.expect("published");
    }
}

fn refused_on<T: std::fmt::Debug>(r: Result<T, Error>, field: &str) {
    assert!(matches!(&r, Err(Error::Validation(e)) if e.iter().any(|f| f.field == field)), "refused on {field}: {r:?}");
}

#[test]
fn the_public_site() {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().thread_stack_size(16 << 20).build().unwrap();
    rt.block_on(async { tokio::spawn(scenario()).await.unwrap() });
}

async fn scenario() {
    let url =
        std::env::var("FERRUM_WEBSITE_SITE_TEST_DATABASE_URL").unwrap_or_else(|_| "postgres://ferrum:ferrum@localhost:5432/ferrum_website_site_test".into());
    let db = Db::connect(DbConfig { url, max_connections: 16, ..Default::default() }).await.expect("connect");
    sqlx::raw_sql("drop schema if exists public cascade; create schema public;").execute(&db.pool).await.expect("reset");
    let mut packages = ferrum_domain::packages().expect("platform packages");
    packages.extend(ferrum_app_website::packages().expect("app packages"));
    packages.extend(
        ferrum_meta::catalog::load_embedded(&[("forms", "package.yaml", FORMS_PACKAGE), ("forms", "entities/forms.yaml", FORMS_ENTITIES)]).expect("forms"),
    );
    let catalog = Catalog::from_packages(packages).expect("catalog");
    let mut hooks = HookRegistry::default();
    ferrum_domain::register(&mut hooks);
    ferrum_app_website::register(&mut hooks);
    let k = Kernel::boot(db, catalog, hooks, KernelSettings::default(), None).await.expect("boot");
    ferrum_domain::maintenance(&k).await.expect("partitions");
    let site = k.hooks.site("website").cloned().expect("the website's site handler");
    let w = World { k, site };
    // The two races first, both run before either is judged, so a failure names each.
    let mut raced = menu_one_level_holds_under_racing_saves(&w).await;
    raced.extend(redirects_one_step_holds_under_racing_saves(&w).await);
    assert!(raced.is_empty(), "{raced:#?}");
    own_css_is_set_through_its_operation_and_served_as_parsed(&w).await;
    blocks_render_escaped(&w).await;
    form_block_frames_the_workspaces_published_form(&w).await;
    nested_menu_one_level(&w).await;
    menu_parent_rules(&w).await;
    redirect_one_hop_301_or_302(&w).await;
    slug_change_makes_redirect(&w).await;
    redirect_loop_refused_and_other_sites_by_administrators_only(&w).await;
    noindex_and_templates_left_out_of_sitemap(&w).await;
    html_lang_dir_from_page(&w).await;
    brand_css_linked(&w).await;
    views_counted_bots_not_and_a_flood_adds_no_rows(&w).await;
    use_template_makes_a_draft(&w).await;
    another_workspaces_pages_never_served(&w).await;
}

/// Two saves racing in two transactions: page A put under B while B is put under C. Exactly one is refused, and the menu
/// stays one level deep. Twelve rounds, each on fresh pages.
async fn menu_one_level_holds_under_racing_saves(w: &World) -> Vec<String> {
    let ws = w.workspace("race-menu", false).await;
    let mut bad = Vec::new();
    for round in 0..12 {
        let mut ids = Vec::new();
        for n in ["a", "b", "c"] {
            ids.push(uid(&w.page(&ws.editor, json!({"title": format!("Race {round} {n}"), "in_menu": true})).await["id"]));
        }
        let (a, b, c) = (ids[0], ids[1], ids[2]);
        let put = |page: Uuid, under: Uuid| {
            let (k, who) = (w.k.clone(), ws.editor.clone());
            tokio::spawn(async move { k.update(&who, "WebPage", page, obj(json!({"menu_parent": under})), None).await })
        };
        let (x, y) = (put(a, b), put(b, c));
        let (x, y) = (x.await.unwrap(), y.await.unwrap());
        let parent = |p: Uuid| {
            let (k, who) = (w.k.clone(), ws.editor.clone());
            async move { k.get(&who, "WebPage", p, false).await.unwrap()["menu_parent"].as_str().map(str::to_string) }
        };
        let (pa, pb) = (parent(a).await, parent(b).await);
        match (x.is_ok(), y.is_ok()) {
            (true, true) => bad.push(format!("menu round {round}: both saves went through")),
            (false, false) => bad.push(format!("menu round {round}: both refused: {x:?} {y:?}")),
            _ => {}
        }
        if pa.is_some() && pb.is_some() {
            bad.push(format!("menu round {round}: two levels stored ({pa:?} under {pb:?})"));
        }
    }
    bad
}

/// Two redirects racing in two transactions, /a → /b and /b → /a: exactly one is refused, and no chain is stored.
async fn redirects_one_step_holds_under_racing_saves(w: &World) -> Vec<String> {
    let ws = w.workspace("race-hops", false).await;
    let mut bad = Vec::new();
    for round in 0..12 {
        let (a, b) = (format!("/r{round}-a"), format!("/r{round}-b"));
        let add = |from: String, to: String| {
            let (k, who) = (w.k.clone(), ws.editor.clone());
            tokio::spawn(async move { k.insert(&who, "WebRedirect", obj(json!({"from_path": from, "to": to}))).await })
        };
        let (x, y) = (add(a.clone(), b.clone()), add(b.clone(), a.clone()));
        let (x, y) = (x.await.unwrap(), y.await.unwrap());
        match (x.is_ok(), y.is_ok()) {
            (true, true) => bad.push(format!("redirect round {round}: both saved: a loop")),
            (false, false) => bad.push(format!("redirect round {round}: both refused: {x:?} {y:?}")),
            _ => {}
        }
        let stored = w.find(&ws.editor, "WebRedirect", &[ferrum_kernel::Filter::eq("from_path", b.as_str())], None, 1).await.unwrap().len()
            + w.find(&ws.editor, "WebRedirect", &[ferrum_kernel::Filter::eq("from_path", a.as_str())], None, 1).await.unwrap().len();
        if stored != 1 {
            bad.push(format!("redirect round {round}: {stored} redirects stored"));
        }
    }
    bad
}

/// The site's own stylesheet goes in only through `set_own_css`, which keeps the sheet as parsed and written again;
/// what `/assets/custom.css` serves is exactly that, never the text given; a refusal names why and changes nothing.
async fn own_css_is_set_through_its_operation_and_served_as_parsed(w: &World) {
    let ws = w.workspace("owncss", false).await;
    let p = w.page(&ws.editor, json!({"title": "Styled"})).await;
    w.publish(&ws.editor, &p).await;
    let settings = w.k.insert(&ws.editor, "WebsiteSettings", obj(json!({"site_title": "Styled"}))).await.unwrap();
    let sid = uid(&settings["id"]);
    // what_is_served_is_what_was_parsed
    let given = "/* ours */\n.hero  h2 { color : #123 }\n@media (max-width: 600px) { .hero { display : none } }\n";
    let done = w.k.run_action(&ws.editor, "WebsiteSettings", sid, "set_own_css", json!({"css": given})).await.expect("set");
    let sheet = done["css"].as_str().expect("the sheet kept").to_string();
    assert_eq!(sheet, ".hero h2{color:#123;}@media (max-width:600px){.hero{display:none;}}");
    let (status, headers, served) = w.get_as(ws.tenant, "/assets/custom.css", "Mozilla").await.unwrap();
    assert_eq!((status, served.as_str()), (200, sheet.as_str()));
    assert!(headers.contains(&("x-content-type-options".into(), "nosniff".into())), "{headers:?}");
    let html = w.get(ws.tenant, "/styled").await.unwrap();
    assert!(html.contains(r#"<link rel="stylesheet" href="/assets/custom.css?v="#), "{html}");
    // Refused with the reason; the sheet served stays as it was.
    for (bad, why) in
        [("@import url(/x.css);", "@import is not taken"), ("input[value^=a]{background:url(https://evil.test/a)}", "is not one of the site's own files")]
    {
        let r = w.k.run_action(&ws.editor, "WebsiteSettings", sid, "set_own_css", json!({"css": bad})).await;
        assert!(matches!(&r, Err(Error::Validation(e)) if e.iter().any(|f| f.field == "css" && f.message.contains(why))), "{bad}: {r:?}");
    }
    assert_eq!(w.get(ws.tenant, "/assets/custom.css").await.unwrap(), sheet);
    // The record's own update cannot put text there past the parser.
    let _ = w.k.update(&ws.editor, "WebsiteSettings", sid, obj(json!({"custom_css": "@import url(https://evil.test/x.css);"})), None).await;
    assert_eq!(w.get(ws.tenant, "/assets/custom.css").await.unwrap(), sheet);
    // Empty: none, and no link.
    w.k.run_action(&ws.editor, "WebsiteSettings", sid, "set_own_css", json!({"css": ""})).await.unwrap();
    assert!(matches!(w.get(ws.tenant, "/assets/custom.css").await, Err(Error::NotFound(_))));
    assert!(!w.get(ws.tenant, "/styled").await.unwrap().contains("custom.css"));
}

/// Words, addresses and Markdown of blocks come out escaped; a script link, a plain-http picture and a far button
/// are dropped where they could still be saved.
async fn blocks_render_escaped(w: &World) {
    let ws = w.workspace("blocks", false).await;
    let evil = r#""><script>alert(1)</script>"#;
    let p = w
        .page(
            &ws.editor,
            json!({"title": "Blocks", "blocks": [
                {"kind": "Text", "heading": evil, "text": "Hi <img src=x onerror=alert(2)> [x](javascript:alert(3))"},
                {"kind": "Image", "image": "https://cdn.acme.test/a.png", "image_alt": evil, "heading": evil},
                {"kind": "Call to Action", "heading": "Go", "button_label": evil, "button_url": "/contact"},
                {"kind": "Hero", "heading": "Big", "text": "**Bold**"},
            ]}),
        )
        .await;
    w.publish(&ws.editor, &p).await;
    let html = w.get(ws.tenant, "/blocks").await.expect("page");
    assert!(!html.contains("<script") && !html.contains("onerror") && !html.contains("javascript:"), "{html}");
    assert!(html.contains("&lt;script&gt;") && html.contains(r#"alt="&quot;&gt;&lt;script&gt;"#), "{html}");
    assert!(html.contains(r#"<a class="button" href="/contact">"#) && html.contains("<strong>Bold</strong>"), "{html}");
    // Pictures and buttons the site would not draw are refused when saved, with the block named.
    for bad in [
        json!({"kind": "Image", "image": "http://acme.test/a.png"}),
        json!({"kind": "Image", "image": "/api/site/other/dam/w/0190f3a0-0000-7000-8000-000000000001"}),
        json!({"kind": "Text", "button_url": "javascript:alert(1)", "button_label": "x"}),
        json!({"kind": "Form", "form": "a/b"}),
    ] {
        refused_on(w.k.insert(&ws.editor, "WebPage", obj(json!({"title": format!("Bad {bad}"), "blocks": [bad]}))).await, "blocks");
    }
    let many: Vec<Value> = (0..61).map(|_| json!({"kind": "Text", "text": "x"})).collect();
    refused_on(w.k.insert(&ws.editor, "WebPage", obj(json!({"title": "Many", "blocks": many}))).await, "blocks");
}

/// A Form block frames the workspace's own form on its own address; a page naming a form that is not published (or
/// Internal) is refused when published, with the address named; without Forms the block draws nothing.
async fn form_block_frames_the_workspaces_published_form(w: &World) {
    let ws = w.workspace("formy", true).await;
    let mut tx = w.k.db.system_tx().await.unwrap();
    sqlx::query("insert into tenant_host (host, tenant_id, kind, target, is_primary, verify_token, verified_at) values ('erp.formy.test', $1, 'custom', 'shell', true, 'x', now())")
        .bind(ws.tenant)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let page = w.page(&ws.editor, json!({"title": "Contact", "blocks": [{"kind": "Form", "heading": "Write to us", "form": "contact"}]})).await;
    let refused = w.k.run_action(&ws.editor, "WebPage", uid(&page["id"]), "publish", Value::Null).await;
    assert!(
        matches!(&refused, Err(Error::Validation(e)) if e.iter().any(|f| f.message.contains("no form with the address contact is published"))),
        "{refused:?}"
    );
    w.k.insert(&ws.admin, "Form", obj(json!({"title": "Inside", "slug": "contact", "status": "Published", "access": "Internal"}))).await.unwrap();
    assert!(w.k.run_action(&ws.editor, "WebPage", uid(&page["id"]), "publish", Value::Null).await.is_err(), "an Internal form is not public");
    let form = w.find(&ws.admin, "Form", &[ferrum_kernel::Filter::eq("slug", "contact")], None, 1).await.unwrap().remove(0);
    w.k.update(&ws.admin, "Form", uid(&form["id"]), obj(json!({"access": "Public"})), None).await.unwrap();
    w.publish(&ws.editor, &page).await;
    let html = w.get(ws.tenant, "/contact").await.expect("page");
    assert!(
        html.contains(r#"<iframe class="form-frame" src="https://erp.formy.test/forms/f/formy/contact?embed=1" title="Write to us" loading="lazy" data-ferrum-embed></iframe>"#),
        "{html}"
    );
    assert!(!html.contains("style="), "no inline style: the site's policy allows none: {html}");
    // The editor's preview frames it on the editor's own address.
    assert!(w.preview(&ws.editor, &page["id"]).await.contains(r#"src="/forms/f/formy/contact?embed=1""#));

    // A workspace without Forms: the block draws nothing, and the page is not published with it.
    let bare = w.workspace("noforms", false).await;
    let page = w.page(&bare.editor, json!({"title": "Contact", "blocks": [{"kind": "Form", "form": "contact"}]})).await;
    assert!(!w.preview(&bare.editor, &page["id"]).await.contains("iframe"));
    refused_on(w.k.run_action(&bare.editor, "WebPage", uid(&page["id"]), "publish", Value::Null).await, "blocks");
}

/// One level: a page under a top page sits in that page's sub-menu; a page under one taken back is not shown; links
/// of the settings sit among the pages by their order.
async fn nested_menu_one_level(w: &World) {
    let ws = w.workspace("menus", false).await;
    let about = w.page(&ws.editor, json!({"title": "About", "in_menu": true, "menu_order": 10})).await;
    let team = w.page(&ws.editor, json!({"title": "Team", "in_menu": true, "menu_parent": about["id"]})).await;
    let hidden = w.page(&ws.editor, json!({"title": "Hidden", "in_menu": true})).await;
    let orphan = w.page(&ws.editor, json!({"title": "Orphan", "in_menu": true, "menu_parent": hidden["id"]})).await;
    for p in [&about, &team, &orphan] {
        w.publish(&ws.editor, p).await;
    }
    w.k.insert(&ws.editor, "WebsiteSettings", obj(json!({"menu_links": [{"label": "Shop", "url": "https://shop.acme.test", "menu_order": 5}]}))).await.unwrap();
    let html = w.get(ws.tenant, "/about").await.unwrap();
    let nav = &html[html.find("<nav").unwrap()..html.find("</nav>").unwrap()];
    assert_eq!(
        nav,
        r#"<nav aria-label="Site"><ul><li><a href="https://shop.acme.test">Shop</a></li><li><details><summary>About</summary><ul><li><a href="/about">About</a></li><li><a href="/team">Team</a></li></ul></details></li><li><a href="/blog">Blog</a></li></ul>"#
    );
    refused_on(
        w.k.update(
            &ws.editor,
            "WebsiteSettings",
            uid(&w.find(&ws.editor, "WebsiteSettings", &[], None, 1).await.unwrap()[0]["id"]),
            obj(json!({"menu_links": [{"label": "x", "url": "javascript:alert(1)"}]})),
            None,
        )
        .await,
        "menu_links",
    );
}

/// The six ways a page is refused a place under another.
async fn menu_parent_rules(w: &World) {
    let ws = w.workspace("parents", false).await;
    let top = w.page(&ws.editor, json!({"title": "Top", "in_menu": true})).await;
    let child = w.page(&ws.editor, json!({"title": "Child", "in_menu": true, "menu_parent": top["id"]})).await;
    let out = w.page(&ws.editor, json!({"title": "Out"})).await;
    let other_top = w.page(&ws.editor, json!({"title": "Other top", "in_menu": true})).await;
    let tpl = w.page(&ws.admin, json!({"title": "Tpl", "is_template": true})).await;
    let set = |p: &Doc, v: Value| w.k.update(&ws.editor, "WebPage", uid(&p["id"]), obj(v), None);
    refused_on(set(&top, json!({"menu_parent": top["id"]})).await, "menu_parent"); // itself
    refused_on(set(&out, json!({"menu_parent": child["id"]})).await, "menu_parent"); // two levels
    refused_on(set(&child, json!({"menu_parent": out["id"]})).await, "menu_parent"); // parent not in the menu
    refused_on(set(&out, json!({"menu_parent": tpl["id"]})).await, "menu_parent"); // parent a template
    refused_on(set(&top, json!({"menu_parent": other_top["id"]})).await, "menu_parent"); // pages are under it
    refused_on(w.k.update(&ws.admin, "WebPage", uid(&tpl["id"]), obj(json!({"in_menu": true})), None).await, "in_menu"); // a template
}

/// An address no page has is led on in one step: 301 when permanent, 302 otherwise; a page at the address wins.
async fn redirect_one_hop_301_or_302(w: &World) {
    let ws = w.workspace("hops", false).await;
    let new = w.page(&ws.editor, json!({"title": "New"})).await;
    w.publish(&ws.editor, &new).await;
    w.k.insert(&ws.editor, "WebRedirect", obj(json!({"from_path": "/old/page/", "to": "/new"}))).await.unwrap();
    w.k.insert(&ws.editor, "WebRedirect", obj(json!({"from_path": "/promo", "to": "/new", "permanent": false}))).await.unwrap();
    let (status, headers, _) = w.get_as(ws.tenant, "/old/page", "Mozilla").await.unwrap();
    assert_eq!(status, 301);
    assert!(headers.contains(&("location".into(), "/new".into())), "{headers:?}");
    let (status, headers, _) = w.get_as(ws.tenant, "/promo", "Mozilla").await.unwrap();
    assert_eq!((status, headers.iter().any(|(k, v)| k == "cache-control" && v == "no-store")), (302, true));
    // A page published at a redirect's address wins it back: the redirect goes.
    let promo = w.page(&ws.editor, json!({"title": "Promo"})).await;
    w.publish(&ws.editor, &promo).await;
    assert_eq!(w.get_as(ws.tenant, "/promo", "Mozilla").await.unwrap().0, 200);
    assert!(w.find(&ws.editor, "WebRedirect", &[ferrum_kernel::Filter::eq("from_path", "/promo")], None, 1).await.unwrap().is_empty());
    assert!(matches!(w.get(ws.tenant, "/nowhere").await, Err(Error::NotFound(_))));
    assert!(matches!(w.get(ws.tenant, &format!("/{}", "a".repeat(301))).await, Err(Error::NotFound(_))));
}

/// A published page whose address changes leaves a redirect from the old one; those that led to the old one now lead
/// to the new one (still one step); changing it back takes the redirect away.
async fn slug_change_makes_redirect(w: &World) {
    let ws = w.workspace("moves", false).await;
    let p = w.page(&ws.editor, json!({"title": "Pricing"})).await;
    w.publish(&ws.editor, &p).await;
    w.k.insert(&ws.editor, "WebRedirect", obj(json!({"from_path": "/prices", "to": "/pricing"}))).await.unwrap();
    w.k.update(&ws.editor, "WebPage", uid(&p["id"]), obj(json!({"slug": "plans"})), None).await.unwrap();
    let (status, headers, _) = w.get_as(ws.tenant, "/pricing", "Mozilla").await.unwrap();
    assert!(status == 301 && headers.contains(&("location".into(), "/plans".into())), "{status} {headers:?}");
    let (_, headers, _) = w.get_as(ws.tenant, "/prices", "Mozilla").await.unwrap();
    assert!(headers.contains(&("location".into(), "/plans".into())), "one step: {headers:?}");
    w.k.update(&ws.editor, "WebPage", uid(&p["id"]), obj(json!({"slug": "pricing"})), None).await.unwrap();
    assert_eq!(w.get_as(ws.tenant, "/pricing", "Mozilla").await.unwrap().0, 200);
    let (_, headers, _) = w.get_as(ws.tenant, "/plans", "Mozilla").await.unwrap();
    assert!(headers.contains(&("location".into(), "/pricing".into())), "{headers:?}");
    // A draft's address was never public: changing it leaves nothing.
    let d = w.page(&ws.editor, json!({"title": "Draft one"})).await;
    w.k.update(&ws.editor, "WebPage", uid(&d["id"]), obj(json!({"slug": "draft-two"})), None).await.unwrap();
    assert!(matches!(w.get(ws.tenant, "/draft-one").await, Err(Error::NotFound(_))));
}

/// No chain, so no loop; never the platform's own paths or another host by `//` or `/\`; another site only when an
/// administrator names it.
async fn redirect_loop_refused_and_other_sites_by_administrators_only(w: &World) {
    let ws = w.workspace("loops", false).await;
    let r = |from: &str, to: &str| obj(json!({"from_path": from, "to": to}));
    w.k.insert(&ws.editor, "WebRedirect", r("/a", "/b")).await.unwrap();
    refused_on(w.k.insert(&ws.editor, "WebRedirect", r("/b", "/a")).await, "to");
    refused_on(w.k.insert(&ws.editor, "WebRedirect", r("/b", "/c")).await, "from_path");
    refused_on(w.k.insert(&ws.editor, "WebRedirect", r("/z", "/a")).await, "to");
    refused_on(w.k.insert(&ws.editor, "WebRedirect", r("/x", "/x")).await, "to");
    for bad_from in ["/_shell/v1/embed.js", "//evil.test", "/\\evil.test", "/a b"] {
        refused_on(w.k.insert(&ws.editor, "WebRedirect", r(bad_from, "/b2")).await, "from_path");
    }
    for bad_to in ["//evil.test", "/\\evil.test", "http://evil.test", "javascript:alert(1)"] {
        refused_on(w.k.insert(&ws.editor, "WebRedirect", r("/y", bad_to)).await, "to");
    }
    refused_on(w.k.insert(&ws.editor, "WebRedirect", r("/shop", "https://shop.example.test")).await, "to");
    let shop = w.k.insert(&ws.admin, "WebRedirect", r("/shop", "https://shop.example.test")).await.expect("an administrator names another site");
    // An editor may still change what else it says, but not where it goes.
    w.k.update(&ws.editor, "WebRedirect", uid(&shop["id"]), obj(json!({"permanent": false})), None).await.unwrap();
    refused_on(w.k.update(&ws.editor, "WebRedirect", uid(&shop["id"]), obj(json!({"to": "https://evil.test"})), None).await, "to");
    let (status, headers, _) = w.get_as(ws.tenant, "/shop", "Mozilla").await.unwrap();
    assert!(status == 302 && headers.contains(&("location".into(), "https://shop.example.test".into())), "{headers:?}");
}

/// Hidden pages carry `noindex` and are left out of the sitemap with templates; a template is not served at all.
async fn noindex_and_templates_left_out_of_sitemap(w: &World) {
    let ws = w.workspace("seo", false).await;
    let hid = w
        .page(
            &ws.editor,
            json!({"title": "Thanks", "noindex": true, "seo_title": "Thank you!", "canonical_url": "https://www.acme.test/thank-you", "og_image": "/files/public/0190f3a0-0000-7000-8000-000000000001"}),
        )
        .await;
    w.publish(&ws.editor, &hid).await;
    let tpl = w.page(&ws.admin, json!({"title": "Landing Template", "is_template": true})).await;
    assert!(w.k.run_action(&ws.admin, "WebPage", uid(&tpl["id"]), "publish", Value::Null).await.is_err(), "a template is never published");
    let html = w.get(ws.tenant, "/thanks").await.unwrap();
    for tag in [
        r#"<meta name="robots" content="noindex">"#,
        "<title>Thank you! · Acme seo</title>",
        r#"<link rel="canonical" href="https://www.acme.test/thank-you">"#,
        r#"<meta property="og:image" content="https://www.acme.test/files/public/0190f3a0-0000-7000-8000-000000000001">"#,
        r#"<meta property="og:title" content="Thanks">"#,
    ] {
        assert!(html.contains(tag), "{tag} in {html}");
    }
    refused_on(w.k.update(&ws.editor, "WebPage", uid(&hid["id"]), obj(json!({"canonical_url": "http://x.test"})), None).await, "canonical_url");
    refused_on(w.k.update(&ws.editor, "WebPage", uid(&hid["id"]), obj(json!({"og_image": "data:image/png;base64,AA"})), None).await, "og_image");
    let map = w.get(ws.tenant, "/sitemap.xml").await.unwrap();
    assert!(!map.contains("thanks") && !map.contains("landing"), "{map}");
    assert!(matches!(w.get(ws.tenant, "/landing-template").await, Err(Error::NotFound(_))), "a template answers as an unknown address");
}

/// A page's language is its `<html lang dir>`; a page and its translations name each other for search engines.
async fn html_lang_dir_from_page(w: &World) {
    let ws = w.workspace("langs", false).await;
    let en = w.page(&ws.editor, json!({"title": "Hello", "language": "en"})).await;
    let ar = w.page(&ws.editor, json!({"title": "Marhaba", "language": "ar", "translation_of": en["id"]})).await;
    for p in [&en, &ar] {
        w.publish(&ws.editor, p).await;
    }
    let html = w.get(ws.tenant, "/marhaba").await.unwrap();
    assert!(html.starts_with(r#"<!doctype html><html lang="ar" dir="rtl">"#), "{html}");
    for tag in [
        r#"<link rel="alternate" hreflang="ar" href="https://www.acme.test/marhaba">"#,
        r#"<link rel="alternate" hreflang="en" href="https://www.acme.test/hello">"#,
    ] {
        assert!(html.contains(tag), "{tag} in {html}");
    }
    assert!(w.get(ws.tenant, "/hello").await.unwrap().starts_with(r#"<!doctype html><html lang="en" dir="ltr">"#));
    assert!(w.page_try(&ws.editor, json!({"title": "Again", "language": "ar", "translation_of": en["id"]})).await.is_err(), "one translation per language");
    refused_on(w.page_try(&ws.editor, json!({"title": "Deep", "language": "hi", "translation_of": ar["id"]})).await, "translation_of");
    refused_on(w.page_try(&ws.editor, json!({"title": "Same", "language": "en", "translation_of": en["id"]})).await, "language");
    refused_on(w.page_try(&ws.editor, json!({"title": "Bad", "language": "not a tag"})).await, "language");
}

impl World {
    /// Records of `entity` as `who` reads them.
    async fn find(&self, who: &Ctx, entity: &str, filters: &[ferrum_kernel::Filter], _sort: Option<&str>, limit: u32) -> Result<Vec<Doc>, Error> {
        let mut tx = self.k.db.tenant_tx(who.tenant).await?;
        let out = self.k.find_tx(who, &mut tx, entity, filters, None, limit).await?;
        tx.commit().await?;
        Ok(out)
    }

    async fn page_try(&self, who: &Ctx, v: Value) -> Result<Doc, Error> {
        self.k.insert(who, "WebPage", obj(v)).await
    }
}

/// The workspace's default brand is linked on every page; the one the settings name wins.
async fn brand_css_linked(w: &World) {
    let ws = w.workspace("brands", false).await;
    let p = w.page(&ws.editor, json!({"title": "Home"})).await;
    w.publish(&ws.editor, &p).await;
    assert!(!w.get(ws.tenant, "/home").await.unwrap().contains("/_brand/"), "no brand: the site's own look");
    let def =
        w.k.insert(&ws.admin, "Brand", obj(json!({"brand_name": "Acme", "primary_color": "#3157d5", "is_default": true, "footer_text": "Acme & Co"})))
            .await
            .unwrap();
    let html = w.get(ws.tenant, "/home").await.unwrap();
    assert!(html.contains(&format!(r#"<link rel="stylesheet" href="/_brand/brands/{}.css?v="#, def["id"].as_str().unwrap())), "{html}");
    assert!(html.contains("<footer>Acme &amp; Co</footer>"), "{html}");
    let other = w.k.insert(&ws.admin, "Brand", obj(json!({"brand_name": "Sub", "primary_color": "#aa0000"}))).await.unwrap();
    w.k.insert(&ws.editor, "WebsiteSettings", obj(json!({"brand": other["id"], "layout": "Wide"}))).await.expect("a Site Editor picks a brand");
    let html = w.get(ws.tenant, "/home").await.unwrap();
    assert!(html.contains(&format!("/_brand/brands/{}.css?v=", other["id"].as_str().unwrap())) && html.contains(r#"<body class="layout-wide">"#), "{html}");
}

/// Each page served to a person adds one to its day; crawlers, assets, the sitemap, redirects and unknown addresses
/// add nothing, so a flood of made-up addresses makes no row; the report sums them.
async fn views_counted_bots_not_and_a_flood_adds_no_rows(w: &World) {
    let ws = w.workspace("views", false).await;
    let p = w.page(&ws.editor, json!({"title": "Popular"})).await;
    w.publish(&ws.editor, &p).await;
    w.k.insert(&ws.editor, "WebRedirect", obj(json!({"from_path": "/old", "to": "/popular"}))).await.unwrap();
    for _ in 0..2 {
        w.get(ws.tenant, "/popular").await.unwrap();
    }
    w.get(ws.tenant, "/popular/").await.unwrap();
    w.get_as(ws.tenant, "/popular", "Mozilla/5.0 (compatible; Googlebot/2.1)").await.unwrap();
    for path in ["/assets/site.css", "/sitemap.xml", "/robots.txt", "/old"] {
        w.get(ws.tenant, path).await.unwrap();
    }
    for n in 0..50 {
        assert!(w.get(ws.tenant, &format!("/made-up-{n}")).await.is_err());
    }
    w.get(ws.tenant, "/").await.unwrap();
    let counted = || async {
        let mut tx = w.k.db.tenant_tx(ws.tenant).await.unwrap();
        let rows: Vec<(String, i64)> = sqlx::query_as("select path, views from website_view order by path").fetch_all(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
        rows
    };
    let rows = counted().await;
    assert_eq!(rows, vec![("/".to_string(), 1), ("/popular".to_string(), 3)]);
    let report = ferrum_app_website::reports().into_iter().next().expect("a report");
    let out = report.run(&w.k, &ws.editor, "site_views", &HashMap::new()).await.unwrap();
    assert_eq!(out["rows"][0], json!({"path": "/popular", "views": 3}));
    // Upkeep keeps 400 days.
    let mut tx = w.k.db.tenant_tx(ws.tenant).await.unwrap();
    sqlx::query("update website_view set day = day - 401 where path = '/'").execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    ferrum_app_website::maintenance(&w.k, ws.tenant).await.unwrap();
    assert_eq!(counted().await, vec![("/popular".to_string(), 3)]);
}

/// A template makes a draft with its text and blocks, made with the caller's own right to make pages.
async fn use_template_makes_a_draft(w: &World) {
    let ws = w.workspace("tpls", false).await;
    let seeded = w.find(&ws.editor, "WebPage", &[ferrum_kernel::Filter::eq("slug", "template-about")], None, 1).await.unwrap();
    let tpl = seeded.first().expect("the seeded templates");
    let made = w.k.run_action(&ws.editor, "WebPage", uid(&tpl["id"]), "use_template", json!({"title": "About Acme"})).await.expect("made");
    let page = w.k.get(&ws.editor, "WebPage", uid(&made["page"]), false).await.unwrap();
    assert_eq!((page["status"].as_str(), page["slug"].as_str(), page["is_template"].as_bool().unwrap_or(false)), (Some("Draft"), Some("about-acme"), false));
    assert_eq!(page["blocks"].as_array().map(Vec::len), tpl_blocks(w, &ws, tpl).await);
    let reader = {
        let k = &w.k;
        let role = "Page Reader".to_string();
        k.set_role_permission(&ws.admin, &role, &serde_json::from_value(json!({"entity": "WebPage", "read": true})).unwrap()).await.expect("role");
        let id = k.create_user(&ws.admin, "read@tpls.test", "read@tpls.test", Some("Correct-Horse-9"), &[role], false).await.expect("user");
        let s = k.login("tpls", "read@tpls.test", "Correct-Horse-9").await.expect("login");
        k.load_ctx(ws.tenant, id, Some(s.token_version), "it".into()).await.expect("ctx")
    };
    let refused = w.k.run_action(&reader, "WebPage", uid(&tpl["id"]), "use_template", json!({"title": "Mine"})).await;
    assert!(matches!(refused, Err(Error::Forbidden(_) | Error::NotFound(_))), "{refused:?}");
    assert!(w.k.run_action(&ws.editor, "WebPage", uid(&page["id"]), "use_template", json!({"title": "From a page"})).await.is_err(), "only a template");
}

async fn tpl_blocks(w: &World, ws: &Ws, tpl: &Doc) -> Option<usize> {
    w.k.get(&ws.editor, "WebPage", uid(&tpl["id"]), false).await.unwrap()["blocks"].as_array().map(Vec::len)
}

/// Workspace B's pages, redirects and drafts are never answered on workspace A's site.
async fn another_workspaces_pages_never_served(w: &World) {
    let a = w.workspace("site-a", false).await;
    let b = w.workspace("site-b", false).await;
    let theirs = w.page(&b.editor, json!({"title": "Secret B"})).await;
    w.publish(&b.editor, &theirs).await;
    w.k.insert(&b.editor, "WebRedirect", obj(json!({"from_path": "/b-old", "to": "/secret-b"}))).await.unwrap();
    assert!(w.get(b.tenant, "/secret-b").await.is_ok());
    assert!(matches!(w.get(a.tenant, "/secret-b").await, Err(Error::NotFound(_))));
    assert!(matches!(w.get(a.tenant, "/b-old").await, Err(Error::NotFound(_))));
    assert!(!w.get(a.tenant, "/sitemap.xml").await.unwrap().contains("secret-b"));
    assert!(!w.get(a.tenant, "/").await.unwrap().contains("Secret B"));
}
