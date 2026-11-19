# Changelog

## 0.3.2

Package `website` 0.2.0 gains, before its first release: the field `WebsiteSettings.custom_css` (read only) and the action `set_own_css`. New dependency `cssparser` 0.38 (MPL-2.0, already in the tree through ammonia). `ui/dist` rebuilt with Ferrum-Frontend `aa75069`.

- **Own CSS, from an allow-list.** `set_own_css {css}` parses the sheet, keeps only style rules, `@media`, `@supports`, `@keyframes` and `@font-face`, and writes it out again from what it parsed; it refuses `@import`, `@namespace`, `@charset` but first, every other at-rule, every address that is not one of the site's own files (public files, the brand's files, small `data:` pictures outside `@font-face`), an address in a selector, `expression(…)`, a declaration that does not parse, and more than 50,000 bytes, 2,000 rules or 8 nested at-rules. Served at `/assets/custom.css` (`text/css`, `nosniff`, the site's policy), linked after the brand's; the record's update cannot write it.
- **The page editor at the page's own address** (`/website/e/WebPage/{id}`): the page's fields in a panel beside its blocks, saved together; *Publish* and *Back to draft* there. No separate blocks page.
- **Proven under racing saves:** the menu stays one level deep and redirects one step long when two saves race (tests that fail with the locks taken out).
- The publish-time form check stays a gate call as the system, not a declared use: a declared use would make Forms required to install Website (see `src/pages.rs`).

## 0.3.1

No database change. Interface only, `ui/dist` rebuilt with Ferrum-Frontend `aa75069`.

- **The blocks editor** at `/website/blocks/{page}`: each block shows the fields its kind uses; blocks move by drag (a handle) or by *Move up* / *Move down*, keeping focus on the button pressed; added by kind and removed; saved with the page through the record's update. Pictures are uploaded as public files or given as `https://` addresses; a Form block offers the published public forms the person can read, and the Form kind is not offered without Forms. The site's rendering of the saved page is beside it.
- **Templates on the home:** *Make a page from it* runs `use_template` with the title typed and opens the draft. Links to *Redirects* and *Site views*; templates are not listed among the pages; the menu shown is its top, as the site's.
- The route table is its own module (`ui/src/routes.tsx`) so tests mount the real one. Words and names of the new screens and fields in Arabic and Hindi (by a model, to be reviewed by native speakers).

## 0.3.0

Package `website` 0.2.0: new entities Redirect, Block (in a page) and Menu Link (in the settings), new fields on Web Page and Website Settings, the table `website_view` (`sql/001_website.sql`), three seeded templates. Existing pages open unchanged (a page's body is no longer required). Needs a platform with the Brand record (core 0.4.0).

- **Blocks.** Text, Image, Hero, Call to Action, Form and Latest Posts after a page's text, up to 60, each drawn escaped; a picture, button address or form name the site would not draw is refused when saved.
- **A form on a page.** A Form block frames a published public form of the workspace from its own address; a page with one is published only while that form is (asked of Forms through the gate, yes or no). Without Forms the block shows nothing.
- **Templates.** Never served; `use_template` makes a draft from one. About, Contact and Landing come with the app.
- **The menu.** One level of sub-menu under a top page (`<details>`, no script), checked so it stays one level when saves race; links of the settings among the pages.
- **Redirects.** One step, never a chain or a loop; from paths of the site only; to another site only when an administrator sets it; made when a published page's address changes.
- **Search engines.** A title for search engines, a preview image, a canonical address, `noindex` (left out of the sitemap), and `hreflang` between a page and its translations.
- **Languages.** A page's language and the page it translates; `<html lang dir>` from it.
- **Brand and layout.** The site shows the workspace's brand (colour, typeface, logo, favicon, footer) and one of three layouts.
- **Page views.** Counted per address and day for people, not crawlers; only addresses that answer a page count, so a flood of made-up ones adds no row. Nothing about the visitor is kept. Report *Site Views*; days older than 400 deleted by upkeep.
- **Not in this release:** own CSS (waits for a CSS parser to be chosen); the visitor's language choosing a translation (waits for the platform).

## 0.2.1

No database or interface change.

- **Publish and Back to Draft carry contracts.** Each says what it changes, what it returns (the status and the address) and what "done" means, so an agent reaches them through the gate as a person does (`scripts/agent_parity.py` finds no action without one).

## 0.2.0

No database or interface change. Needs Ferrum 0.4.0 (apps' upkeep runs one workspace at a time, on whichever replica is free).

- **Upkeep takes one workspace.** `maintenance(k, tenant)` is the platform's per-workspace entry point; Website still has nothing to run by itself.

## 0.1.14

Needs the same platform as 0.1.13. No database change. Interface only, `ui/dist` rebuilt with Ferrum-Frontend `06bd1a2`.

- The two routes to the screens apps no longer have (`apps/:app/settings`, `x/:app/:page`) are gone; the interface lint has no baseline file any more. The page keeps its own start (5 seconds of freshness).

## 0.1.13

Rebuilt with Ferrum-Frontend 9e7190c, the design system as tuned after the landed build was reviewed. The left menu's icons are chosen so that none repeats on more than three rows and each fits its row; their drawings load beside the first page instead of inside it, so the page starts lighter; a row that scrolls sideways fades at the edge where there is more, in every browser and with reduced motion; a printed record leaves out empty fields and keeps its totals with the table; menus and popovers sit inside the page's landmarks for assistive technology; a selection bar folds what does not fit into More on a narrow screen. Nothing else changes.

## 0.1.12

The interface follows the upgraded design system, rebuilt with Ferrum-Frontend 76c4fad. Text uses the kit's type roles (nothing under 12px) and controls its height tokens; loading shows skeletons shaped like what is coming; empty, no matches, denied, offline and failed are told apart; status colours come from stored state ids. The interface check now runs the UI lint against an empty baseline. The home has a page head with Settings and New page as buttons, and a slug reads left to right in a right-to-left page.

## 0.1.11

Rebuilt with Ferrum-Frontend 43d7779: the names of its models, fields and manifest in Arabic and Hindi (40 each in ui/src/locales/names, which the platform serves to every page of the workspace, so a record's labels, the launcher and the bar read in the person's language), and the sentences its pages still showed in English (split around a link or a figure, built in code from a template or word by word) whole and translated (9 words each in ui/src/locales). Translated by the model, to be reviewed by native speakers.

## 0.1.10

The assistant knows what its records mean in its pages (contributes.shell.assistant): their states, what counts and where figures come from, with questions it suggests asking there.

## 0.1.9

Rebuilt with Ferrum-Frontend 389b2af: its own words carry built-in Arabic and Hindi (10 words each, ui/src/locales, translated by the model and to be reviewed by native speakers), fetched for the page's language only. The labels it keeps in objects, its dialogs, its page titles and the words it keeps in variables go through t() as well, and the names and sizes it shows as they are keep their order right to left. Its pages are drawn once their words are in, and its entry carries only the parts of the kit it uses.

## 0.1.8

Rebuilt with Ferrum-Frontend 1f5981d: an action's link, code or token shown once is kept in a dialog to copy, and on a page that reads right to left, English left untranslated keeps its own order. Its own words go through the kit's t() (13 places), so a workspace's own words for its language reach them, and Admin, Interface words lists them under it (dist/words.json).

## 0.1.7

Rebuilt with Ferrum-Frontend 8a1c2e7: a workspace's own words for its interface (Admin, Interface words) reach its pages as they load; an attachment field that holds a picture shows it; and a value another record already has is said so, where the form said the record had been changed by someone else. Its own screens' margins, paddings, borders, alignment and insets go by the line's start and end, so they mirror right to left as the kit's do; what it places by coordinates (boards, timelines, charts, handles) stays as it was.

## 0.1.6

Rebuilt with Ferrum-Frontend 9d80f4f: right to left, its pages mirror as they read (icons that point along the line, margins and alignment by the line's start and end, arrow keys along it), and the records pages' words are in Arabic and Hindi.

## 0.1.5

Rebuilt with Ferrum-Frontend 204b713: a new record's box whose default is 1 shows ticked, as it is saved. An action's dialog says every refusal, the errors on what the action made (another record's rows) among them, and an action with parameters and a confirmation asks once.

## 0.1.4

Rebuilt with Ferrum-Frontend b39fffb: its pages show times on a person's own clock when they chose one on their Account page, while today and due dates stay on the workspace's day.

## 0.1.3

Its site handler says it serves pages at the root of a `site:website` host, so Ferrum's Domains offers this app's site, and only apps that serve one (Ferrum PR 85).

## 0.1.2

From a browser walk on a running server (an About page and a post with a table, strikethrough, a
script and a script link, a picture by https and one by http, published; the settings; the pages,
blog, sitemap and robots.txt as the site serves them):

- **Preview:** the editor's preview is now the site's own rendering of the page's text, from
  `GET /api/app/website/preview/{id}` (drafts included, for whoever may read the page). Before, it
  was the kit's Markdown view, which showed tables, strikethrough and pictures as plain text.
- **The site's name:** without a title of its own, the site is called by the workspace's name, as
  the settings say, rather than "Website".
- **Link previews:** each page carries `og:title`, `og:description` (its summary), `og:site_name`,
  `og:type` and, on its host, `og:url`.
- Rebuilt with Ferrum-Frontend d350579: times on the workspace's clock.

## 0.1.1

Rebuilt with Ferrum-Frontend 20d08e5: a record page reads an empty number or check as the server does, so an action whose condition needs one shows, and a workspace's settings open as their one record.

## 0.1.0

The workspace's public website (Ferrum's FERRUM-ECOSYSTEM-006).

- **Pages and blog posts**, written in Markdown:
  - each lives at an address made from its title (lower-case letters, digits and dashes), and never one of the site's own paths;
  - it is published with the time, or taken back to a draft.
  - Drafts are never served.
- **The site**, served on a `site:website` host and rendered on the server, with no script:
  - the home page the settings name, else the latest posts;
  - the blog, and each published page or post;
  - a menu of the pages put in it, in their order;
  - a sitemap, robots.txt, a canonical address and a description for search engines and link previews.
- **Safe Markdown:** raw HTML never passes; links go to the web, mail or the site's own pages; images are shown from https:// addresses only.
- **Its own interface at /website/:** the pages and posts, published ones first, the menu as the site shows it, and a preview, drafts included.
