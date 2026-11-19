// What the blocks editor decides: the kinds of block, the fields each kind shows (as the site draws it, src/site.rs),
// moving a block, and what is saved: only the fields of a block's kind, so the record holds what the site shows.

export const KINDS = ["Text", "Image", "Hero", "Call to Action", "Form", "Latest Posts"] as const;
export type Kind = (typeof KINDS)[number];

export interface Block {
  kind: Kind;
  heading?: string | null;
  text?: string | null;
  image?: string | null;
  image_alt?: string | null;
  button_label?: string | null;
  button_url?: string | null;
  form?: string | null;
  posts?: number | null;
}

export type Field = Exclude<keyof Block, "kind">;

const FIELDS: Record<Kind, Field[]> = {
  Text: ["heading", "text"],
  Image: ["image", "image_alt", "heading"],
  Hero: ["heading", "text", "image", "image_alt", "button_label", "button_url"],
  "Call to Action": ["heading", "text", "button_label", "button_url"],
  Form: ["heading", "form"],
  "Latest Posts": ["heading", "posts"],
};

/** The fields a block of `kind` shows, in order. */
export const fieldsOf = (kind: Kind): Field[] => FIELDS[kind] ?? [];

/** A page has at most this many blocks (src/pages.rs BLOCKS_MAX). */
export const BLOCKS_MAX = 60;

/** `list` with the item at `i` moved `by` places (kept inside the list). */
export function move<T>(list: T[], i: number, by: number): T[] {
  const to = Math.max(0, Math.min(list.length - 1, i + by));
  if (to === i || i < 0 || i >= list.length) return list;
  const out = list.slice();
  const [item] = out.splice(i, 1);
  out.splice(to, 0, item);
  return out;
}

/** A new block of `kind`. */
export const blank = (kind: Kind): Block => (kind === "Latest Posts" ? { kind, posts: 5 } : { kind });

/** The rows saved: each block's kind and the non-empty fields of its kind. */
export function forSave(blocks: Block[]): Block[] {
  return blocks.map((b) => {
    const row: Block = { kind: b.kind };
    for (const f of fieldsOf(b.kind)) {
      const v = b[f];
      if (v !== undefined && v !== null && v !== "") (row as unknown as Record<string, unknown>)[f] = v;
    }
    return row;
  });
}
