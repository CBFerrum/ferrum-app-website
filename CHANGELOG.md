# Changelog

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
