// What the editors' page decides: the order pages are listed in, and the menu as the site shows it.

export interface Row {
  id: string;
  [k: string]: unknown;
}

const title = (r: Row) => String(r.title ?? "").toLowerCase();

/** Published first, the newest first; then drafts by title. */
export function ordered(rows: Row[]): Row[] {
  const live = rows.filter((r) => r.status === "Published").sort((a, b) => String(b.published_at ?? "").localeCompare(String(a.published_at ?? "")));
  const drafts = rows.filter((r) => r.status !== "Published").sort((a, b) => title(a).localeCompare(title(b)));
  return [...live, ...drafts];
}

/** The site's menu's top: published pages in it that are under no other and translate none, by their order, then
 * title (as the server orders it). */
export function menu(rows: Row[]): Row[] {
  return rows
    .filter((r) => r.status === "Published" && r.in_menu && !r.menu_parent && !r.translation_of && !r.is_template)
    .sort((a, b) => Number(a.menu_order ?? 100) - Number(b.menu_order ?? 100) || title(a).localeCompare(title(b)));
}
