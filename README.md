# Ferrum Website

**A native app for [Ferrum](https://github.com/CBFerrum/Ferrum), the open-source, AI-first business platform.**

## Overview

The workspace's public website: pages and blog posts written in Markdown, with a menu and a home page.

- **Where it is served:** on its own host (`site:website`). An administrator gives the workspace's domain to the Website app.
- **How it is rendered:** on the server, so search engines and link previews read every page, and no page runs a script.

| At a glance | |
|---|---|
| 📦 Version | `0.1.0` |
| 🧱 Platform | Ferrum `>=0.3.2, <0.4` |
| 🖥️ Interface | its own, at `/website/`; the public site on its host |
| 🗃️ Records | 2 entities in 1 package |
| 👥 Roles | Site Editor |

## How it works

| Record | What it is |
|---|---|
| `WebPage` | A page or a post. Fields: `title`, `slug`, `kind` (Page or Post), `summary`, `body` (Markdown), `in_menu`, `menu_order`. Set by its actions: `status` (Draft or Published) and `published_at`. |
| `WebsiteSettings` | `site_title`, `tagline`, `home_page`, `blog_title`, `footer`. |

How `slug` is set:
- It is made from the title when it is not given: lower-case letters, digits and dashes, at most 120 characters.
- `/blog` and `/assets` are the site's own, so a page cannot take them.
- A title with no letters or digits to make one from needs one given.

**Actions:** `publish` (with the time) and `unpublish` (back to a draft).

**The public site:**

| Address | What it serves |
|---|---|
| `/` | The home page the settings name, else the latest posts. |
| `/blog` | The posts, newest first. |
| `/{slug}` | A published page or post. |
| `/sitemap.xml` | Every published page. |
| `/robots.txt` | Points to the sitemap. |
| `/assets/site.css` | The stylesheet. |

- Drafts and unknown addresses are not found.
- The text is escaped.
- Markdown keeps tables and strikethrough, and drops raw HTML.
- Links go to `https://`, `http://`, `mailto:`, `#…` or `/…`. A link elsewhere keeps only its text.
- Images are shown from `https://` addresses only.

## Limits

- **Images:** they are https:// addresses. There is no media library of its own; Drive's public files may be linked.
- **Pages:** one level of pages, no nested menus, no themes beyond the light and dark stylesheet, and no forms. Web forms are the Forms app's job.
- **Preview:** the preview in `/website/` shows the page's text as the site renders it, with this app's styles rather than the site's stylesheet.

## Changelog

| Version | Highlights |
|---|---|
| **0.1.10** | The assistant knows its records. |
| **0.1.9** | Rebuilt with Ferrum-Frontend 5b22e7b: its own words in Arabic and Hindi. |
| **0.1.8** | Rebuilt with Ferrum-Frontend 6537d65: once-shown answers kept; English isolated right to left; its own words translatable by a workspace. |
| **0.1.7** | Rebuilt with Ferrum-Frontend d5d98f2: a workspace's own interface words; pictures in attachment fields; a duplicate value said so; its own screens mirror right to left. |
| **0.1.6** | Rebuilt with Ferrum-Frontend 63280d3: right to left, pages mirror; the records pages' words in Arabic and Hindi. |
| **0.1.5** | Rebuilt with Ferrum-Frontend a64416a: a box whose default is 1 shows ticked; action dialogs say every refusal and ask once. |
| **0.1.4** | Rebuilt with Ferrum-Frontend a76295e: times on a person's own clock when they chose one. |
| **0.1.3** | Domains offers its site: the handler says it serves pages. |
| **0.1.2** | From a browser walk: the editor's preview is the site's own rendering; a site without a title is the workspace's name; pages carry what link previews read. Rebuilt with Ferrum-Frontend 24c3951. |
| **0.1.1** | Rebuilt with Ferrum-Frontend bb60115: empty numbers read as the server reads them; settings open as their record. |
| **0.1.0** | Pages and blog posts in Markdown, a menu and a home page, served on the workspace's own host and rendered on the server. |

## Develop

```bash
cargo test   # needs PostgreSQL: FERRUM_WEBSITE_TEST_DATABASE_URL
cd ui && npm run typecheck && npm test && npm run build   # with Ferrum-Frontend at ../../ferrum/web
```

## License

AGPL-3.0-only.
