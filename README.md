<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/banner-dark.svg">
  <img alt="Ferrum Website: pages and a blog in Markdown, served on your own domain" src="docs/assets/banner-light.svg" width="100%">
</picture>

<br>

[![Version](https://img.shields.io/badge/version-0.3.0-0d9488?style=flat-square)](CHANGELOG.md)
[![Licence: AGPL-3.0](https://img.shields.io/badge/licence-AGPL--3.0-blue?style=flat-square)](LICENSE)
[![Ferrum platform](https://img.shields.io/badge/runs%20on-Ferrum-2f5bea?style=flat-square)](https://github.com/CBFerrum/Ferrum)
![Rust](https://img.shields.io/badge/Rust-server-b7410e?style=flat-square&logo=rust)
![PostgreSQL](https://img.shields.io/badge/PostgreSQL-data-336791?style=flat-square&logo=postgresql&logoColor=white)
![React](https://img.shields.io/badge/React-TypeScript-0ea5e9?style=flat-square&logo=react&logoColor=white)
![Tests](https://img.shields.io/badge/tests-11%20Rust%20%2B%202%20UI-16a34a?style=flat-square)

### A public website and blog for your workspace, written in Markdown and served on your own domain.

Website is for a company that needs a clear public site: a home page, a few pages, a menu and a blog, without a separate website builder to log in to. Editors write pages in Markdown and publish them; the Ferrum server renders each page as plain HTML on the workspace's own host.

[💡 Why](#-why-website) · [📸 Screenshots](#-screenshots) · [✨ Features](#-features) · [📊 Compare](#-how-it-compares) · [📍 Roadmap](#-roadmap) · [🚀 Install](#-install) · [🔧 Develop](#-develop)

</div>

## 💡 Why Website

- ⚡ **Rendered on the server.** Search engines and link previews read every page, and no page runs a script.
- 🔒 **Safe by construction.** Text is escaped, raw HTML is dropped from Markdown, links go only to safe addresses and images only from `https://`.
- ✍️ **Draft, preview, publish.** Write in Markdown, preview a draft as the site will show it, and publish or take back to draft with one action.
- 🌐 **Your domain.** An administrator gives the workspace's domain to the Website app, and the site is served at its root.
- 🔍 **Search-ready.** A `sitemap.xml`, a `robots.txt`, canonical links, and the title and summary link previews read.
- 🌓 **Light and dark.** The stylesheet follows the reader's setting, with no web fonts.
- 🧩 **Same workspace as the rest.** Pages have permissions, audit, history and an API like every other record, and AI agents work with them under the same rules.
- 🔓 **Open source and self-hosted.** AGPL, with no per-site fee.

## 🎯 At a glance

| | |
|---|---|
| 📦 Version | `0.3.0` |
| 🧱 Kind | Own interface at `/website/` for editors; the public site on its own host |
| 🗃️ What it adds | 3 entities (Web Page, Redirect, Website Settings) and 2 tables in records (Block, Menu Link) in the `website` package; a table of page views |
| 👥 Roles | 1: Site Editor |
| 📊 Reports and print formats | 1 report: Site Views |
| 🔗 Depends on | Ferrum `>=0.4.0, <0.5`; a domain given to it (Domains) to serve on |
| 🤝 Works with | Domains (the host) |
| 🌍 Languages | The editor's interface in English, Arabic and Hindi (translated by a model, to be reviewed by native speakers). Each public page says its own language (`<html lang dir>`, right to left for Arabic, Hebrew, Persian and Urdu) and names its translations for search engines |
| 🧪 Tests | 11 Rust tests (two against a real PostgreSQL, one of them walking 13 checks of the public site), 2 interface tests |
| 📄 Licence | AGPL-3.0-only |

## 📸 Screenshots

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/website-home-dark.webp">
  <img alt="Website home" src="docs/assets/screens/website-home-light.webp" width="100%">
</picture>

<p align="center"><b>🌐 Website home</b><br><sub>See the menu as the site shows it, list the pages and posts, and preview one as it renders.</sub></p>

<table>
<tr>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/website-web-page-list-dark.webp"><img alt="Web pages list" src="docs/assets/screens/website-web-page-list-light.webp"></picture>
<p><b>📄 Web pages</b><br><sub>Browse, search and filter pages and posts by kind and status.</sub></p>
</td>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/website-web-page-detail-dark.webp"><img alt="One web page" src="docs/assets/screens/website-web-page-detail-light.webp"></picture>
<p><b>✍️ One page</b><br><sub>Write the Markdown, set its address, menu entry and summary, then publish.</sub></p>
</td>
</tr>
</table>

<p align="center">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/website-home-phone-dark.webp"><img alt="Website home on a phone" src="docs/assets/screens/website-home-phone-light.webp" width="260"></picture>
&nbsp;&nbsp;
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/website-web-page-list-phone-dark.webp"><img alt="Web pages on a phone" src="docs/assets/screens/website-web-page-list-phone-light.webp" width="260"></picture>
</p>

<p align="center"><b>📱 On a phone</b><br><sub>The editors' home and the pages list on a narrow screen.</sub></p>

## ✨ Features

### ✍️ Pages and posts

- **Web Page.** One record for a page and for a blog post. A title (up to 200 characters), an address (`slug`), a kind (Page or Post), a summary (up to 300 characters, for search engines, link previews and the blog list), a Markdown body (up to 100,000 characters) shown first, then its blocks, an *In the Menu* box, a menu order and the page it sits *Under* in the menu.
- **Blocks.** After its text a page shows up to 60 blocks, in order: **Text** (a heading and Markdown), **Image** (a public file, this workspace's DAM web address or an `https://` picture, with what it shows and a caption), **Hero** (a large heading, text, a picture and a button), **Call to Action** (a heading, text and a button), **Form** (a published Forms form, shown in a frame) and **Latest Posts** (1 to 20 of the newest posts). A picture, a button's address or a form's name the site would not show is refused when saved, naming the block.
- **Forms on a page.** A Form block names the form's address in Forms. The page is published only while that form is published and public; otherwise *Publish* says "no form with the address … is published". Website asks Forms through the platform's gate, as the system, only whether such a form is published (public knowledge), so a Site Editor needs no right in Forms. The frame comes from the workspace's own address, which the platform lets the workspace's addresses frame; without Forms the block shows nothing and a page with one is not published.
- **Templates.** A page marked *Template* is never served and is not in the menu. *Make a Page From It* (the `use_template` operation, needing the right to make pages) makes a draft with its text and blocks and the title given. Three come with the app: About, Contact and Landing.
- **Search engines.** A title for search engines (up to 70 characters), a preview image for link previews (with a large-image card), a canonical address (`https://` only) and *Hide From Search Engines* (`noindex`, and left out of the sitemap).
- **Languages.** A page's language (a tag such as `en`, `hi` or `pt-BR`) and the page it translates: one translation per language, of the original only.
- **Draft and Published.** A new page is a Draft, seen only in the preview by those who may read it. *Publish* sets the status and the time; *Back to Draft* takes it off the site.
- **The address.** Made from the title when empty: lower-case letters, digits and dashes, up to 120 characters. It is unique. `/blog` and `/assets` belong to the site and cannot be taken. A title with no letters or digits to make one from needs an address given.
- **The menu.** Published pages with *In the Menu* ticked, by menu order (lower first) and then title, and the settings' *Links in the Menu* (up to 20: the shop, another site) among them by their order, at most 60 entries, plus a link to the blog. **One level below:** a page *Under* a top page opens in that page's sub-menu (`<details>`, no script). A page is refused a place under itself, under a page that is under another, under a page not in the menu or a template, or when pages are already under it. A page under one taken off the site is not shown.
- **Redirects.** An old address that leads to a new one, permanent (301) or for now (302), served only when no page has the address. One step only: a redirect is refused when it would lead to another redirect's start or start where another leads, so no chain and no loop can form. Never from the platform's own paths (`/_…`), never to another host by `//` or `/\`; to another site (`https://…`) only when an administrator sets it. **When a published page's address changes,** a redirect from the old address is made, and those that led to the old one lead to the new one; a page published at a redirect's address takes it back.
- **Website Settings.** The site's title (the workspace's name when empty), tagline, home page, the blog's heading (*Blog* when empty), a footer (the brand's footer text when empty, else a copyright line with the site's title), the brand, the layout (Classic, Centred or Wide) and the links in the menu.
- **Brand.** The site shows the brand its settings name, else its address's, else the workspace's default (Ferrum's Brand records, kept by administrators): its colour and typeface through the brand's stylesheet, its logo in the header (and its dark-mode logo), its favicon, its footer text and links. A Site Editor may read the brands to pick one.
- **Create menu.** *Web page* in Ferrum's create menu starts a new draft.

### 🌐 The public site

| Address | What it serves |
|---|---|
| `/` | The home page the settings name, else the latest posts |
| `/blog` | The posts, newest first, with their dates and summaries (up to 50) |
| `/{slug}` | A published page or post (not a template); a post shows its date |
| any other address | The redirect from it, once, else not found |
| `/sitemap.xml` | The home, the blog and every published page not hidden from search engines, with the day each was published |
| `/robots.txt` | Allows everything and points to the sitemap |
| `/assets/site.css` | The stylesheet, cached for a year |

- **Drafts, templates and unknown addresses are not found** alike, and the site is read only.
- **Views counted, nothing about the visitor kept.** Each page served to a person adds one to that address's count for the day (the workspace's day); crawlers and link previews (a `User-Agent` naming a bot, crawler, spider or preview), the stylesheet, the sitemap, robots, redirects and unknown addresses add nothing, so a flood of made-up addresses adds no row. No address, cookie or visitor is kept. The **Site Views** report lists the 50 addresses most viewed in the last 30 days; days older than 400 are deleted by upkeep.
- **Link previews and search.** Each page carries its title (or its title for search engines) with the site's name, the summary as description, a canonical link, its translations (`hreflang`), and Open Graph tags (site name, title, description, address and preview image).
- **No script.** The site's stylesheet follows the reader's light or dark setting and takes the brand's colour and typeface; no font from another site. A page's form frame is the only thing it loads from another address, and only from the workspace's own.
- **Own CSS: not yet.** A Site Editor's own stylesheet waits for a CSS parser to be chosen (see the roadmap); scripts are refused by design, since a site address may later carry a customer's session.
- **Served on the platform's host.** The platform hands every path of a `site:website` host to the Website app with the site's own origin, never the caller's.

### 🔐 Safe Markdown

- **Tables and strikethrough** are kept.
- **Raw HTML is dropped,** and its text is not shown.
- **Links** go to `https://`, `http://`, `mailto:`, `#…` or a path of this site (`/…`, never `//`). A link elsewhere keeps only its text.
- **Images** are shown only from `https://` addresses; otherwise their description stays as text.
- **Escaping.** All text is escaped in HTML and in attributes: page and block words, alternative text, button words, addresses.

### 👁️ The editors' page

- **Home.** The menu as the site shows it, then every page and post: published ones first (newest first), then drafts by title, with a *Post* badge, the status and an *Edit* link.
- **Preview.** Choose a page and see it as the site renders it, drafts included, for those who may read it with their own rights. The preview uses this app's styles, not the site's stylesheet.
- **Settings and New page** are one click away.
- **Honest states.** Loading skeletons, "No pages yet.", and refused, offline and failed told apart.

### 🔐 Roles and fit

| Role | Can do |
|---|---|
| **Site Editor** | Reads, writes, creates, deletes and exports pages; reads, writes, creates and deletes redirects (to another site only if also an administrator); reads, writes and creates the site's settings; reads brands. Publishes, unpublishes and makes pages from templates. |

- **Assistant.** Ferrum's assistant knows pages are Draft or Published and suggests "Which pages are still drafts?" and "What did we publish this month?".
- **Languages.** The editor's interface is in English, Arabic and Hindi with right-to-left mirroring.

## 🧭 How it works

```mermaid
flowchart LR
    ED["Site Editor"] --> WP["Web Page (Draft)"]
    WP -- "publish" --> PUB["Published"]
    ED -- "preview" --> RN["Rendered as the site shows it"]
    PUB --> SRV["Ferrum server renders HTML"]
    DOM["Domain given to the Website app"] --> SRV
    V["Visitor"] --> SRV
    SRV --> V
    SRV --> SM["sitemap.xml and robots.txt"]
```

An editor writes and publishes. A visitor asks the workspace's domain; the platform passes the path to the Website app, which finds the published page, renders its Markdown and returns plain HTML.

```mermaid
flowchart TD
    A["Request for /about"] --> B{"Published page with that slug?"}
    B -- "yes" --> C["Render Markdown, drop raw HTML, keep safe links"]
    B -- "no" --> D["Not found"]
    C --> E["Layout: menu, title, summary, canonical, Open Graph"]
```

## 🧱 Data model

| Entity | What it is | Key fields or states |
|---|---|---|
| Web Page | A page or a blog post at `/{slug}` | title, slug, kind (Page / Post), summary, body (Markdown), blocks, Draft / Published, published at, in menu, menu order, under, language, translation of, title for search engines, preview image, canonical address, hide from search engines, template |
| Block (in a page) | One part of a page after its text | kind (Text, Image, Hero, Call to Action, Form, Latest Posts), heading, text, image, image description, button label and address, form, posts |
| Redirect | An old address leading to a new one | from, to, permanent |
| Website Settings | The site's one settings record | site title, tagline, home page, blog title, footer, brand, layout, links in the menu |
| Menu Link (in the settings) | An address in the menu | label, address, menu order |

| Action | Who | What it does |
|---|---|---|
| `publish` | Site Editor | Publishes a draft and records the time. |
| `unpublish` | Site Editor | Takes a published page back to a draft. |
| `use_template` | Whoever may make pages | Makes a draft page from a template with the title given. |

Each carries a contract (what it changes, returns and means when done), so an agent builds and publishes a whole site through the same operations. Report: Site Views. Upkeep: deletes page views older than 400 days. Table `website_view` (per address and day: a count only).

## 🔌 Integrations

| With | How | Needs |
|---|---|---|
| Domains | An administrator gives the workspace's domain to the Website app; the app says it serves pages | Domains and a domain you control |
| Commerce | A Commerce storefront is served the same way, on its own host; a menu link leads to it | Commerce installed |
| Forms | A Form block frames a published public form | Forms installed |
| Brand | The site shows the workspace's brand: colour, typeface, logo, favicon, footer | A Brand record (administrators) |
| Search engines and link previews | `sitemap.xml`, `robots.txt`, canonical and Open Graph tags | Nothing extra |
| Ferrum's assistant and MCP | Pages are records available to agents under each person's rights | Nothing extra |

## 📊 How it compares

The leaders are website builders: Webflow, Wix, WordPress, Squarespace and Odoo Website. Ferrum Website is a small, safe, text-first site: it does not try to be a visual builder.

| Capability | Ferrum Website | Webflow | Wix | WordPress | Squarespace | Odoo Website |
|---|:-:|:-:|:-:|:-:|:-:|:-:|
| Pages and a menu | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Blog with posts | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Draft, preview and publish | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Markdown writing | ✅ | ❌ | ❌ | 🟡 | 🟡 | ❌ |
| Visual drag-and-drop builder | ❌ | ✅ | ✅ | 🟡 | ✅ | ✅ |
| Templates and themes | 🟡 | ✅ | ✅ | ✅ | ✅ | ✅ |
| Custom CSS and code | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Media library | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Forms built in | ✅ | ✅ | ✅ | 🟡 | ✅ | ✅ |
| Online store | 🟡 | ✅ | ✅ | 🟡 | ✅ | ✅ |
| Nested menus | 🟡 | ✅ | ✅ | ✅ | ✅ | ✅ |
| Custom domain | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Sitemap and robots | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Link previews (Open Graph) | ✅ | ✅ | ✅ | 🟡 | ✅ | ✅ |
| Per-page SEO fields and redirects | ✅ | ✅ | ✅ | 🟡 | ✅ | ✅ |
| Multi-language sites | 🟡 | ✅ | ✅ | 🟡 | 🟡 | ✅ |
| Site analytics | 🟡 | 🟡 | ✅ | 🟡 | ✅ | ✅ |
| Plugins and app marketplace | ❌ | 🟡 | ✅ | ✅ | 🟡 | ✅ |
| Server-rendered with no script on the page | ✅ | ❌ | ❌ | 🟡 | ❌ | ❌ |
| Same data as CRM, shop and accounting | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ |
| Open source | ✅ | ❌ | ❌ | ✅ | ❌ | 🟡 |
| Self-hosted | ✅ | ❌ | ❌ | ✅ | ❌ | ✅ |
| No per-site subscription | ✅ | ❌ | ❌ | 🟡 | ❌ | 🟡 |

Notes: Ferrum's templates are pages to start from, with the workspace's brand and three layouts, not a theme marketplace (🟡). Nested menus go one level below the top (🟡). Multi-language: pages say their language and name their translations; choosing a translation by the visitor's language waits for the platform (🟡). Analytics are page views per address and day, kept without anything about the visitor (🟡: no referrers or sessions). 🟡 for Ferrum's online store means the separate Commerce app has its own storefront; Website does not sell. WordPress's Markdown, forms, store, SEO and multi-language support come through plugins. Odoo's Community edition is open source, and its Website builder features in Enterprise differ, hence 🟡. Hosted builders' own pages usually run scripts of their own.

### 🏆 Where Website stands out

- 🔒 **No script, escaped text, safe links.** The public site is plain HTML.
- 🧩 **Part of the workspace.** Pages, permissions and audit are the platform's.
- 💶 **No site subscription** and no vendor to host it.
- 🔓 **Open source and self-hosted.**

### 🚧 Where the leaders are still ahead

- 🎨 **Visual building,** themes and custom CSS.
- 🖼️ **Media library** and image handling.
- 🔎 **Analytics** beyond page views, and choosing a page's language by the visitor's.
- 🧰 **Ecosystem.** Plugins and a large marketplace.

> [!NOTE]
> Leader columns are from public product information as of November 2026 and may change.

## 📍 Roadmap

### ✅ Recently shipped

- [x] **0.3.0** Blocks (text, image, hero, call to action, form, latest posts), templates, one level of sub-menu and menu links, search-engine fields, one-step redirects (made when an address changes), page languages, the workspace's brand and three layouts, and page views without anything about the visitor.
- [x] **0.2.1** Publish and Back to Draft say what they do, for people and agents alike.
- [x] **0.2.0** Upkeep takes one workspace: needs Ferrum 0.4.0.
- [x] **0.1.14** Rebuilt on Ferrum-Frontend 06bd1a2.
- [x] **0.1.13** Rebuilt on the tuned design system: better menu icons, a lighter first page, a visible scroll cue, cleaner print.
- [x] **0.1.12** The redesigned interface: shaped loading, told-apart states.
- [x] **0.1.11** Names in Arabic and Hindi.
- [x] **0.1.10** The assistant knows its records.
- [x] **0.1.3** Domains offers its site: the handler says it serves pages.
- [x] **0.1.2** From a browser walk: the editor's preview is the site's own rendering; a site without a title is the workspace's name; pages carry what link previews read.
- [x] **0.1.0** Pages and blog posts in Markdown, a menu and a home page, served on the workspace's own host and rendered on the server.

### 🔜 Next up

- [ ] **A media library.** Pictures from DAM (waits for DAM's web addresses).
- [ ] **The visitor's language** chooses a page's translation (waits for the platform's language headers).
- [ ] **Own CSS,** parsed and limited to the site's own files (waits for the choice of a CSS parser).
- [ ] **Scheduled publishing.**

### 🔭 Later

- [ ] More than one level of sub-menu.
- [ ] An RSS feed for the blog.
- [ ] Preview in the site's own stylesheet.

## 🚀 Install

It is compiled into the Ferrum server. A workspace administrator adds **Website** from **Apps** (`/store/`), and gives the workspace's domain to the Website app (Domains) so the site is served at its root.

First steps:

1. Give **Site Editor** to the people who write the site.
2. Fill **Website Settings**: title, tagline, footer, and a home page once one exists.
3. Write a page, tick *In the Menu* for the ones to list, preview it, then **Publish**.
4. Write posts (kind *Post*) for the blog.

Limits to know:
- Images are `https://` addresses; there is no media library of its own.
- One level of pages, no nested menus, no forms, and no themes beyond the stylesheet.

## 🔧 Develop

```bash
cargo test   # needs PostgreSQL: FERRUM_WEBSITE_TEST_DATABASE_URL
cd ui && npm run typecheck && npm test && npm run build   # with Ferrum-Frontend at ../../ferrum/web
```

CI creates a non-superuser role (so row-level security applies) and the database `ferrum_website_test`, and sets `FERRUM_WEBSITE_TEST_DATABASE_URL=postgres://ferrum:ferrum@localhost:5432/ferrum_website_test`. It runs `cargo fmt -- --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test`, then the interface checks against a Ferrum-Frontend checkout. Markdown goes through `pulldown-cmark` (MIT).

## 📂 Project layout

```text
app.yaml            the manifest: id, version, what it adds to Ferrum
packages/website/   entities and roles
src/                pages (slugs, publish) and the public site and preview endpoint
assets/site.css     the site's stylesheet, light and dark
ui/src/             Home.tsx (pages, menu, preview), pages.ts (order and menu), locales
tests/              website_it.rs: integration tests against a real PostgreSQL
docs/               PROVENANCE.md
```

## 📜 Changelog

Every version is in [CHANGELOG.md](CHANGELOG.md).

## 🤝 Contributing

Issues and pull requests are welcome at [github.com/CBFerrum/ferrum-app-website/issues](https://github.com/CBFerrum/ferrum-app-website/issues).

## 📄 License

[AGPL-3.0-only](LICENSE), like the platform.

<div align="center">

<sub>Part of <a href="https://github.com/CBFerrum/Ferrum">Ferrum</a>, the open business platform · Made with 🦀 Rust and 🐘 PostgreSQL</sub>

</div>
