# Changelog

## 0.1.11

Rebuilt with Ferrum-Frontend c59c9a2: the names of its models, fields and manifest in Arabic and Hindi (40 each in ui/src/locales/names, which the platform serves to every page of the workspace, so a record's labels, the launcher and the bar read in the person's language), and the sentences its pages still showed in English (split around a link or a figure, built in code from a template or word by word) whole and translated (9 words each in ui/src/locales). Translated by the model, to be reviewed by native speakers.

## 0.1.10

The assistant knows what its records mean in its pages (contributes.shell.assistant): their states, what counts and where figures come from, with questions it suggests asking there.

## 0.1.9

Rebuilt with Ferrum-Frontend 5b22e7b: its own words carry built-in Arabic and Hindi (10 words each, ui/src/locales, translated by the model and to be reviewed by native speakers), fetched for the page's language only. The labels it keeps in objects, its dialogs, its page titles and the words it keeps in variables go through t() as well, and the names and sizes it shows as they are keep their order right to left. Its pages are drawn once their words are in, and its entry carries only the parts of the kit it uses.

## 0.1.8

Rebuilt with Ferrum-Frontend 6537d65: an action's link, code or token shown once is kept in a dialog to copy, and on a page that reads right to left, English left untranslated keeps its own order. Its own words go through the kit's t() (13 places), so a workspace's own words for its language reach them, and Admin, Interface words lists them under it (dist/words.json).

## 0.1.7

Rebuilt with Ferrum-Frontend d5d98f2: a workspace's own words for its interface (Admin, Interface words) reach its pages as they load; an attachment field that holds a picture shows it; and a value another record already has is said so, where the form said the record had been changed by someone else. Its own screens' margins, paddings, borders, alignment and insets go by the line's start and end, so they mirror right to left as the kit's do; what it places by coordinates (boards, timelines, charts, handles) stays as it was.

## 0.1.6

Rebuilt with Ferrum-Frontend 63280d3: right to left, its pages mirror as they read (icons that point along the line, margins and alignment by the line's start and end, arrow keys along it), and the records pages' words are in Arabic and Hindi.

## 0.1.5

Rebuilt with Ferrum-Frontend a64416a: a new record's box whose default is 1 shows ticked, as it is saved. An action's dialog says every refusal, the errors on what the action made (another record's rows) among them, and an action with parameters and a confirmation asks once.

## 0.1.4

Rebuilt with Ferrum-Frontend a76295e: its pages show times on a person's own clock when they chose one on their Account page, while today and due dates stay on the workspace's day.

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
- Rebuilt with Ferrum-Frontend 24c3951: times on the workspace's clock.

## 0.1.1

Rebuilt with Ferrum-Frontend bb60115: a record page reads an empty number or check as the server does, so an action whose condition needs one shows, and a workspace's settings open as their one record.

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
