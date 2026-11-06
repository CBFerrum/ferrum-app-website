# Changelog

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
