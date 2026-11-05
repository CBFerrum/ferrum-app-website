//! A small company's website (FERRUM-ECOSYSTEM-006) against a real PostgreSQL
//! (`FERRUM_WEBSITE_TEST_DATABASE_URL`, default `postgres://ferrum:ferrum@localhost:5432/ferrum_website_test`;
//! reset first), its public pages asked for as the platform hands a `site:website` host's requests
//! to the package: addresses made from titles and kept off the site's own paths; drafts never
//! served; the home page, the blog, the menu, the sitemap and robots.txt; Markdown without
//! scripts; and who may write.

use ferrum_kernel::secret::SecretKey;
use ferrum_kernel::tenant::NewTenant;
use ferrum_kernel::{Ctx, Db, DbConfig, Doc, Error, HookRegistry, Kernel, KernelSettings, Site, SiteRequest};
use ferrum_meta::Catalog;
use serde_json::{Map, Value, json};
use std::sync::Arc;
use uuid::Uuid;

fn obj(v: Value) -> Doc {
    v.as_object().cloned().unwrap()
}

fn uid(v: &Value) -> Uuid {
    v.as_str().unwrap_or_else(|| panic!("not an id: {v}")).parse().unwrap()
}

/// A public page as the platform asks for it: (status, content type, body), or the error.
async fn get(k: &Kernel, site: &Arc<dyn Site>, tenant: Uuid, path: &str) -> Result<(u16, String, String), Error> {
    let mut headers = Map::new();
    headers.insert("x-ferrum-site-api".into(), json!("/api/site/acme/website"));
    headers.insert("x-ferrum-site-origin".into(), json!("https://www.acme.test"));
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
    let r = site.handle(k, req).await?;
    Ok((r.status, r.content_type, String::from_utf8(r.body).unwrap()))
}

#[test]
fn a_company_website() {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().thread_stack_size(16 << 20).build().unwrap();
    rt.block_on(async { tokio::spawn(scenario()).await.unwrap() });
}

async fn scenario() {
    let url = std::env::var("FERRUM_WEBSITE_TEST_DATABASE_URL").unwrap_or_else(|_| "postgres://ferrum:ferrum@localhost:5432/ferrum_website_test".into());
    let db = Db::connect(DbConfig { url, max_connections: 16, ..Default::default() }).await.expect("connect");
    sqlx::raw_sql("drop schema if exists public cascade; create schema public;").execute(&db.pool).await.expect("reset");
    let mut packages = ferrum_domain::packages().expect("platform packages");
    packages.extend(ferrum_app_website::packages().expect("app packages"));
    let catalog = Catalog::from_packages(packages).expect("catalog");
    let mut hooks = HookRegistry::default();
    ferrum_domain::register(&mut hooks);
    ferrum_app_website::register(&mut hooks);
    let settings = KernelSettings { secret_key: Some(SecretKey::new([7u8; 32])), ..Default::default() };
    let k = Kernel::boot(db, catalog, hooks, settings, None).await.expect("boot");
    ferrum_domain::maintenance(&k).await.expect("partitions");
    let site = k.hooks.site("website").cloned().expect("the website's site handler");
    let tenant = k
        .create_tenant(NewTenant {
            slug: "acme".into(),
            name: "Acme".into(),
            admin_email: "admin@acme.test".into(),
            admin_password: "Correct-Horse-9".into(),
            admin_name: None,
            packages: vec!["website".into()],
            settings: Some(json!({"timezone": "UTC"})),
        })
        .await
        .expect("tenant");
    let session = k.login("acme", "admin@acme.test", "Correct-Horse-9").await.expect("login");
    let admin = k.load_ctx(tenant, session.user, Some(session.token_version), "it".into()).await.expect("ctx");
    let user = |email: &'static str, roles: &'static [&'static str]| {
        let (k, admin) = (k.clone(), admin.clone());
        async move {
            let roles: Vec<String> = roles.iter().map(|r| r.to_string()).collect();
            let id = k.create_user(&admin, email, email, Some("Correct-Horse-9"), &roles, false).await.expect("user");
            let s = k.login("acme", email, "Correct-Horse-9").await.expect("login");
            k.load_ctx(tenant, id, Some(s.token_version), "it".into()).await.expect("ctx")
        }
    };
    let editor: Ctx = user("web@acme.test", &["Site Editor"]).await;
    let other: Ctx = user("sam@acme.test", &[]).await;
    let page = |title: &str, kind: &str, body: &str, extra: Value| {
        let mut v = obj(json!({"title": title, "kind": kind, "body": body}));
        v.extend(obj(extra));
        v
    };

    // ───────────── addresses ─────────────
    let about = k
        .insert(&editor, "WebPage", page("About Us", "Page", "We make **boilers**.", json!({"in_menu": true, "menu_order": 10, "summary": "Who we are"})))
        .await
        .unwrap();
    assert_eq!(about["slug"], "about-us", "made from the title");
    let blog = k.insert(&editor, "WebPage", page("Blog", "Page", "x", json!({}))).await;
    assert!(matches!(&blog, Err(Error::Validation(e)) if e.iter().any(|f| f.field == "slug")), "the site's own path: {blog:?}");
    let hindi = k.insert(&editor, "WebPage", page("नमस्ते", "Page", "x", json!({}))).await;
    assert!(matches!(&hindi, Err(Error::Validation(e)) if e.iter().any(|f| f.field == "slug")), "no letters to make one from: {hindi:?}");
    assert!(k.insert(&editor, "WebPage", page("About us", "Page", "again", json!({}))).await.is_err(), "an address is one page's");
    let script = "Hi <script>alert(1)</script> [x](javascript:alert(1)) <img src=x onerror=alert(2)>\n\n![logo](https://cdn.acme.test/logo.png)";
    let hello = k.insert(&editor, "WebPage", page("Hello <World>", "Post", script, json!({"summary": "Our first post"}))).await.unwrap();
    let secret = k.insert(&editor, "WebPage", page("Secret plans", "Page", "Not yet.", json!({"in_menu": true}))).await.unwrap();
    assert_eq!(hello["slug"], "hello-world");

    // ───────────── drafts are not served ─────────────
    assert!(matches!(get(&k, &site, tenant, "/about-us").await, Err(Error::NotFound(_))), "a draft");
    for id in [&about["id"], &hello["id"]] {
        k.run_action(&editor, "WebPage", uid(id), "publish", Value::Null).await.expect("published");
    }
    let (status, ctype, html) = get(&k, &site, tenant, "/about-us").await.expect("about");
    assert_eq!((status, ctype.as_str()), (200, "text/html; charset=utf-8"));
    assert!(
        html.contains("<h1>About Us</h1>") && html.contains("<strong>boilers</strong>") && html.contains(r#"<meta name="description" content="Who we are">"#),
        "{html}"
    );
    assert!(html.contains(r#"<link rel="canonical" href="https://www.acme.test/about-us">"#) && html.contains(r#"href="/assets/site.css?v="#), "{html}");
    assert!(html.contains(r#"<li><a href="/about-us">About Us</a></li>"#) && !html.contains("Secret plans"), "the menu: published pages only: {html}");
    assert!(matches!(get(&k, &site, tenant, "/secret-plans").await, Err(Error::NotFound(_))));

    // ───────────── a post, its words escaped and its Markdown without scripts ─────────────
    let (_, _, post) = get(&k, &site, tenant, "/hello-world").await.expect("post");
    assert!(post.contains("<h1>Hello &lt;World&gt;</h1>") && post.contains(r#"<p class="when">"#), "{post}");
    assert!(!post.contains("<script") && !post.contains("javascript:") && !post.contains("onerror"), "{post}");
    assert!(post.contains(r#"<img src="https://cdn.acme.test/logo.png" alt="logo""#), "{post}");

    // ───────────── home, blog, sitemap, robots.txt ─────────────
    let (_, _, home) = get(&k, &site, tenant, "/").await.expect("home");
    assert!(home.contains(r#"<a href="/hello-world">Hello &lt;World&gt;</a>"#) && home.contains("Our first post"), "the latest posts: {home}");
    let settings =
        k.insert(&editor, "WebsiteSettings", obj(json!({"site_title": "Acme Boilers", "home_page": about["id"], "footer": "Acme Ltd"}))).await.unwrap();
    assert!(settings.get("id").is_some());
    let (_, _, home) = get(&k, &site, tenant, "/").await.expect("home");
    assert!(
        home.contains("<h1>About Us</h1>") && home.contains("<title>About Us · Acme Boilers</title>") && home.contains("<footer>Acme Ltd</footer>"),
        "{home}"
    );
    let (_, _, posts) = get(&k, &site, tenant, "/blog").await.expect("blog");
    assert!(posts.contains("<h1>Blog</h1>") && posts.contains("/hello-world") && !posts.contains("/about-us\">About Us</a> <time"), "posts only: {posts}");
    let (_, ctype, map) = get(&k, &site, tenant, "/sitemap.xml").await.expect("sitemap");
    assert!(
        ctype.starts_with("application/xml")
            && map.contains("<loc>https://www.acme.test/about-us</loc>")
            && map.contains("hello-world")
            && !map.contains("secret"),
        "{map}"
    );
    let (_, _, robots) = get(&k, &site, tenant, "/robots.txt").await.expect("robots");
    assert!(robots.contains("Sitemap: https://www.acme.test/sitemap.xml"));
    let (_, ctype, _) = get(&k, &site, tenant, "/assets/site.css").await.expect("css");
    assert!(ctype.starts_with("text/css"));

    // Taken back, it leaves the site; addresses that are no page's are not found.
    k.run_action(&editor, "WebPage", uid(&hello["id"]), "unpublish", Value::Null).await.expect("back to draft");
    assert!(matches!(get(&k, &site, tenant, "/hello-world").await, Err(Error::NotFound(_))));
    assert!(matches!(get(&k, &site, tenant, "/Not A Slug").await, Err(Error::NotFound(_))));
    assert!(matches!(get(&k, &site, tenant, "/a/b").await, Err(Error::NotFound(_))));

    // ───────────── who may write ─────────────
    assert!(matches!(k.insert(&other, "WebPage", page("Mine", "Page", "x", json!({}))).await, Err(Error::Forbidden(_))));
    assert!(matches!(k.run_action(&other, "WebPage", uid(&secret["id"]), "publish", Value::Null).await, Err(Error::Forbidden(_) | Error::NotFound(_))));
}
