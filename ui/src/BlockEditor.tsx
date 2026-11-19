// A page's editor, at the page's own address (/website/e/WebPage/{id}): its blocks, each showing the fields its kind uses
// (as the site draws it), moved by drag or by their Move up / Move down buttons; the page's other fields (title, address,
// menu, language, search engines, template) in a panel; all saved together through the record's own update, the same
// operation an agent uses, and published or taken back through its actions. A picture is uploaded as a public file or
// given as an https:// address; a form is named by its address in Forms, with the published forms the person can see
// offered. The preview beside it is the site's own rendering of what is saved.

import { closestCenter, DndContext, type DragEndEvent, KeyboardSensor, PointerSensor, useSensor, useSensors } from "@dnd-kit/core";
import { SortableContext, sortableKeyboardCoordinates, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { type ReactNode, useId, useMemo, useState } from "react";
import { useParams } from "react-router";
import { ApiError, api, docPath, isolate, t } from "@ferrum/client";
import { Button, Field, ReaderSkeleton, StatePanel, toast, useShellTitle } from "@ferrum/kit";
import { FormView, LinkInput, useMetaIndex } from "@ferrum/records";
import { BLOCKS_MAX, type Block, blank, type Field as BlockField, fieldsOf, forSave, type Kind, KINDS, move } from "./blocks";

interface PageDoc {
  id: string;
  title?: string;
  slug?: string;
  status?: string;
  version?: number;
  blocks?: Block[];
  [field: string]: unknown;
}

/** The page's own fields the panel edits, in order, with how each is entered. */
const PAGE_FIELDS = [
  ["title", "text"],
  ["slug", "text"],
  ["kind", "kind"],
  ["summary", "area"],
  ["body", "area"],
  ["in_menu", "check"],
  ["menu_order", "number"],
  ["menu_parent", "page"],
  ["language", "text"],
  ["translation_of", "page"],
  ["seo_title", "text"],
  ["og_image", "text"],
  ["canonical_url", "text"],
  ["noindex", "check"],
  ["is_template", "check"],
] as const;
type PageField = (typeof PAGE_FIELDS)[number][0];

const PAGE_WORDS: Record<PageField, () => string> = {
  title: () => t("Title"),
  slug: () => t("Address"),
  kind: () => t("Kind"),
  summary: () => t("Summary"),
  body: () => t("Text before the blocks (Markdown)"),
  in_menu: () => t("In the menu"),
  menu_order: () => t("Menu order"),
  menu_parent: () => t("Under (in the menu)"),
  language: () => t("Language"),
  translation_of: () => t("Translates"),
  seo_title: () => t("Title for search engines"),
  og_image: () => t("Preview image"),
  canonical_url: () => t("Canonical address"),
  noindex: () => t("Hide from search engines"),
  is_template: () => t("Template"),
};

/** A block in the editor: its row and a key that stays with it while it moves. */
interface Row {
  key: string;
  block: Block;
}

let seq = 0;
const keyed = (block: Block): Row => ({ key: `b${++seq}`, block });

const KIND_WORDS: Record<Kind, () => string> = {
  Text: () => t("Text"),
  Image: () => t("Image"),
  Hero: () => t("Hero"),
  "Call to Action": () => t("Call to Action"),
  Form: () => t("Form"),
  "Latest Posts": () => t("Latest Posts"),
};

const FIELD_WORDS: Record<BlockField, () => string> = {
  heading: () => t("Heading"),
  text: () => t("Text (Markdown)"),
  image: () => t("Picture's address"),
  image_alt: () => t("What the picture shows"),
  button_label: () => t("Button's words"),
  button_url: () => t("Button's address"),
  form: () => t("Form's address name"),
  posts: () => t("How many posts"),
};

function Failed({ error, onRetry }: { error: unknown; onRetry: () => void }) {
  const status = error instanceof ApiError ? error.status : -1;
  return <StatePanel kind={status === 403 ? "denied" : status === 404 ? "empty" : status === 0 ? "offline" : "failed"} error={error} onRetry={onRetry} />;
}

/** A record's page: a web page's own editor, any other record (and a new page) the records frame's form. */
export function Record() {
  const { entity = "", id = "" } = useParams();
  return entity === "WebPage" && id !== "new" ? <PageEditor id={id} /> : <FormView />;
}

function PageEditor({ id }: { id: string }) {
  const q = useQuery({ queryKey: ["website", "page", id], queryFn: ({ signal }) => api.get<PageDoc>(docPath("WebPage", id), { titles: false }, signal) });
  useShellTitle(q.data?.title ? String(q.data.title) : t("Web page"));
  if (q.isPending) return <ReaderSkeleton />;
  if (q.isError) return <Failed error={q.error} onRetry={() => void q.refetch()} />;
  return <Editor key={`${q.data.id}:${q.data.version ?? 0}`} page={q.data} />;
}

function Editor({ page }: { page: PageDoc }) {
  const qc = useQueryClient();
  const index = useMetaIndex();
  const forms = !!index.data?.entities.some((e) => e.name === "Form");
  const [rows, setRows] = useState<Row[]>(() => (page.blocks ?? []).map(keyed));
  const [fields, setFields] = useState<Record<string, unknown>>(() => Object.fromEntries(PAGE_FIELDS.map(([f]) => [f, page[f] ?? null])));
  const [dirty, setDirty] = useState(false);
  const edit = (f: string, v: unknown) => {
    setFields((x) => ({ ...x, [f]: v === "" ? null : v }));
    setDirty(true);
  };
  const [adding, setAdding] = useState<Kind>("Text");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const kinds = KINDS.filter((k) => forms || k !== "Form");
  const sensors = useSensors(useSensor(PointerSensor), useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }));
  const preview = useQuery({
    queryKey: ["website", "preview", page.id, page.version],
    queryFn: ({ signal }) => api.get<{ title: string; html: string }>(`/api/app/website/preview/${page.id}`, {}, signal),
  });

  const change = (i: number, b: Block) => {
    setRows((r) => r.map((x, n) => (n === i ? { ...x, block: b } : x)));
    setDirty(true);
  };
  const shift = (i: number, by: number) => {
    setRows((r) => move(r, i, by));
    setDirty(true);
    // Focus stays with the block's button that was pressed, now at its new place.
    requestAnimationFrame(() => document.getElementById(`${by < 0 ? "up" : "down"}-${rows[i]?.key}`)?.focus());
  };
  const dropped = (e: DragEndEvent) => {
    if (!e.over || e.active.id === e.over.id) return;
    const from = rows.findIndex((r) => r.key === e.active.id);
    const to = rows.findIndex((r) => r.key === e.over?.id);
    if (from >= 0 && to >= 0) {
      setRows((r) => move(r, from, to - from));
      setDirty(true);
    }
  };
  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      await api.patch(docPath("WebPage", page.id), { ...fields, blocks: forSave(rows.map((r) => r.block)) }, { ifMatch: page.version });
      toast(t("Page saved"));
      await qc.invalidateQueries({ queryKey: ["website", "page", page.id] });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };
  // Publish and Back to Draft are the page's own actions; a page with unsaved changes is saved first.
  const act = async (action: "publish" | "unpublish") => {
    setSaving(true);
    setError(null);
    try {
      await api.post(docPath("WebPage", page.id, "actions", action));
      toast(action === "publish" ? t("Published") : t("Back to draft"));
      await qc.invalidateQueries({ queryKey: ["website", "page", page.id] });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="np-page">
      <header className="page-head">
        <div>
          <h1 className="page-title">{isolate(String(page.title ?? ""))}</h1>
          <p className="page-sub">
            <span dir="ltr">/{isolate(String(page.slug ?? ""))}</span> · {page.status === "Published" ? t("Published") : t("Draft")}
          </p>
        </div>
        <div className="page-actions">
          {!page.is_template &&
            (page.status === "Published" ? (
              <Button disabled={dirty || saving} onClick={() => void act("unpublish")} title={dirty ? t("Save first") : undefined}>
                {t("Back to draft")}
              </Button>
            ) : (
              <Button disabled={dirty || saving} onClick={() => void act("publish")} title={dirty ? t("Save first") : undefined}>
                {t("Publish")}
              </Button>
            ))}
          <Button variant="primary" loading={saving} onClick={() => void save()}>
            {t("Save page")}
          </Button>
        </div>
      </header>
      {error && (
        <p className="field-error" role="alert">
          {error}
        </p>
      )}
      <div className="np-columns">
        <div className="stack">
        <section className="card card-pad stack" aria-labelledby="ws-fields">
          <h2 id="ws-fields" className="font-semibold">
            {t("The page")}
          </h2>
          <PageFields values={fields} onChange={edit} />
        </section>
        <section className="card card-pad stack" aria-labelledby="ws-blocks">
          <h2 id="ws-blocks" className="font-semibold">
            {t("Blocks ({length})", { length: rows.length })}
          </h2>
          {rows.length === 0 && <p className="faint">{t("No blocks yet: the page shows its text only.")}</p>}
          <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={dropped}>
            <SortableContext items={rows.map((r) => r.key)} strategy={verticalListSortingStrategy}>
              <ol className="stack ws-blocks">
                {rows.map((r, i) => (
                  <Card
                    key={r.key}
                    id={r.key}
                    n={i + 1}
                    last={i === rows.length - 1}
                    onUp={() => shift(i, -1)}
                    onDown={() => shift(i, 1)}
                    onRemove={() => {
                      setRows((x) => x.filter((_, n) => n !== i));
                      setDirty(true);
                    }}
                    kind={r.block.kind}
                  >
                    <BlockFields block={r.block} forms={forms} onChange={(b) => change(i, b)} />
                  </Card>
                ))}
              </ol>
            </SortableContext>
          </DndContext>
          <div className="row" style={{ gap: 8, flexWrap: "wrap" }}>
            <label className="field-label" htmlFor="ws-add">
              {t("Add a block")}
            </label>
            <select id="ws-add" value={adding} onChange={(e) => setAdding(e.target.value as Kind)}>
              {kinds.map((k) => (
                <option key={k} value={k}>
                  {KIND_WORDS[k]()}
                </option>
              ))}
            </select>
            <Button
              disabled={rows.length >= BLOCKS_MAX}
              onClick={() => {
                setRows((r) => [...r, keyed(blank(adding))]);
                setDirty(true);
              }}
            >
              {t("Add")}
            </Button>
          </div>
        </section>
        </div>
        <section className="card card-pad stack" aria-labelledby="ws-preview">
          <h2 id="ws-preview" className="font-semibold">
            {t("Preview (as saved)")}
          </h2>
          {preview.isPending ? (
            <ReaderSkeleton />
          ) : preview.isError ? (
            <Failed error={preview.error} onRetry={() => void preview.refetch()} />
          ) : (
            // The server's HTML: its text escaped, raw HTML dropped, and only safe links, pictures and form frames kept.
            <div className="ws-site" dangerouslySetInnerHTML={{ __html: preview.data.html }} />
          )}
        </section>
      </div>
    </div>
  );
}

function Card({ id, n, last, kind, onUp, onDown, onRemove, children }: { id: string; n: number; last: boolean; kind: Kind; onUp: () => void; onDown: () => void; onRemove: () => void; children: ReactNode }) {
  const { attributes, listeners, setNodeRef, transform, transition } = useSortable({ id });
  return (
    <li ref={setNodeRef} className="card card-pad stack" style={{ transform: CSS.Transform.toString(transform), transition }} aria-label={t("Block {n}: {kind}", { n, kind: KIND_WORDS[kind]() })}>
      <div className="row" style={{ gap: 8, flexWrap: "wrap", alignItems: "center" }}>
        <button type="button" className="icon-btn" aria-label={t("Drag block {n}", { n })} {...attributes} {...listeners}>
          ⠿
        </button>
        <strong>{t("{n}. {kind}", { n, kind: KIND_WORDS[kind]() })}</strong>
        <Button size="sm" id={`up-${id}`} disabled={n === 1} onClick={onUp} aria-label={t("Move block {n} up", { n })}>
          {t("Move up")}
        </Button>
        <Button size="sm" id={`down-${id}`} disabled={last} onClick={onDown} aria-label={t("Move block {n} down", { n })}>
          {t("Move down")}
        </Button>
        <Button size="sm" variant="ghost" onClick={onRemove} aria-label={t("Remove block {n}", { n })}>
          {t("Remove")}
        </Button>
      </div>
      {children}
    </li>
  );
}

/** Uploads `file` as a public file (`/files/public/{id}`): a page shows it to anyone. */
async function uploadPublic(file: File): Promise<string> {
  const fd = new FormData();
  fd.append("file", file);
  fd.append("public", "true");
  return (await api.post<{ url: string }>("/api/files", fd)).url;
}

interface FormRow {
  id: string;
  slug?: string;
  title?: string;
}

function BlockFields({ block, forms, onChange }: { block: Block; forms: boolean; onChange: (b: Block) => void }) {
  const base = useId();
  const set = (f: BlockField, v: string | number | null) => onChange({ ...block, [f]: v === "" ? null : v });
  // The published public forms this person can read, offered as they type; someone who reads none types the address.
  const published = useQuery({
    queryKey: ["website", "forms"],
    enabled: forms && block.kind === "Form",
    queryFn: ({ signal }) =>
      api.get<{ rows: FormRow[] }>(docPath("Form"), { fields: "title,slug", filters: JSON.stringify([["status", "=", "Published"], ["access", "=", "Public"]]), limit: 200 }, signal).then((r) => r.rows),
  });
  const [uploading, setUploading] = useState(false);
  const fields = useMemo(() => fieldsOf(block.kind), [block.kind]);
  if (block.kind === "Form" && !forms) return <p className="faint">{t("Forms is not installed: this block shows nothing.")}</p>;
  return (
    <div className="stack" data-kind={block.kind}>
      {fields.map((f) => {
        const id = `${base}-${f}`;
        const value = block[f] ?? "";
        if (f === "text")
          return (
            <Field key={f} label={FIELD_WORDS[f]()}>
              <textarea id={id} rows={4} value={String(value)} onChange={(e) => set(f, e.target.value)} />
            </Field>
          );
        if (f === "posts")
          return (
            <Field key={f} label={FIELD_WORDS[f]()}>
              <input id={id} type="number" min={1} max={20} value={String(value)} onChange={(e) => set(f, e.target.value ? Number(e.target.value) : null)} />
            </Field>
          );
        if (f === "image")
          return (
            <Field key={f} label={FIELD_WORDS[f]()} hint={t("A public file uploaded here, or an https:// address.")}>
              <div className="row" style={{ gap: 8, flexWrap: "wrap" }}>
                <input id={id} dir="ltr" value={String(value)} onChange={(e) => set(f, e.target.value)} />
                <label className="btn btn-sm">
                  {uploading ? t("Uploading…") : t("Upload a picture")}
                  <input
                    type="file"
                    accept="image/png,image/jpeg,image/webp,image/gif,image/avif"
                    className="sr-only"
                    disabled={uploading}
                    onChange={async (e) => {
                      const file = e.target.files?.[0];
                      if (!file) return;
                      setUploading(true);
                      try {
                        set(f, await uploadPublic(file));
                      } catch (err) {
                        toast(err instanceof Error ? err.message : String(err), "error");
                      } finally {
                        setUploading(false);
                      }
                    }}
                  />
                </label>
              </div>
            </Field>
          );
        if (f === "form") {
          const list = `${base}-forms`;
          return (
            <Field key={f} label={FIELD_WORDS[f]()} hint={t("The form's address in Forms (its last part). A page is published only when the form is.")}>
              <input id={id} dir="ltr" list={list} value={String(value)} onChange={(e) => set(f, e.target.value.trim().toLowerCase())} />
              <datalist id={list}>
                {(published.data ?? []).map((r) => (
                  <option key={r.id} value={r.slug ?? ""}>
                    {r.title}
                  </option>
                ))}
              </datalist>
            </Field>
          );
        }
        return (
          <Field key={f} label={FIELD_WORDS[f]()}>
            <input id={id} dir={f === "button_url" ? "ltr" : undefined} value={String(value)} onChange={(e) => set(f, e.target.value)} />
          </Field>
        );
      })}
    </div>
  );
}

/** The page's own fields, each saved with the blocks. */
function PageFields({ values, onChange }: { values: Record<string, unknown>; onChange: (f: string, v: unknown) => void }) {
  const base = useId();
  return (
    <div className="stack">
      {PAGE_FIELDS.map(([f, how]) => {
        const id = `${base}-${f}`;
        const v = values[f];
        const label = PAGE_WORDS[f]();
        if (how === "check")
          return (
            <label key={f} className="row" style={{ gap: 8 }}>
              <input id={id} type="checkbox" checked={!!v} onChange={(e) => onChange(f, e.target.checked)} />
              {label}
            </label>
          );
        if (how === "page")
          return (
            <Field key={f} label={label} htmlFor={id}>
              <LinkInput id={id} to="WebPage" value={(v as string | null) ?? null} onChange={(next) => onChange(f, next)} label={label} />
            </Field>
          );
        if (how === "kind")
          return (
            <Field key={f} label={label}>
              <select id={id} value={String(v ?? "Page")} onChange={(e) => onChange(f, e.target.value)}>
                <option value="Page">{t("Page")}</option>
                <option value="Post">{t("Post")}</option>
              </select>
            </Field>
          );
        if (how === "area")
          return (
            <Field key={f} label={label}>
              <textarea id={id} rows={f === "body" ? 6 : 2} value={String(v ?? "")} onChange={(e) => onChange(f, e.target.value)} />
            </Field>
          );
        if (how === "number")
          return (
            <Field key={f} label={label}>
              <input id={id} type="number" min={0} max={10000} value={v === null || v === undefined ? "" : String(v)} onChange={(e) => onChange(f, e.target.value === "" ? null : Number(e.target.value))} />
            </Field>
          );
        return (
          <Field key={f} label={label}>
            <input id={id} dir={f === "slug" || f.endsWith("_url") || f === "og_image" || f === "language" ? "ltr" : undefined} value={String(v ?? "")} onChange={(e) => onChange(f, e.target.value)} />
          </Field>
        );
      })}
    </div>
  );
}
