-- Website: page views per address and day, counted by the site (src/site.rs) for each page it serves to a person
-- (not to a crawler). No address, no cookie, nothing about who viewed: only the count. Only addresses that answer a
-- page are counted, so the rows stay at most one per page and day. Upkeep deletes days older than 400.
create table website_view (
  tenant_id uuid not null references tenant(id) on delete cascade,
  path text not null check (length(path) between 1 and 300),
  day date not null,
  views bigint not null default 0 check (views >= 0),
  primary key (tenant_id, path, day)
);
alter table website_view enable row level security;
alter table website_view force row level security;
create policy tenant_isolation on website_view
  using (tenant_id = (select ferrum_tenant()) or (select ferrum_is_system()))
  with check (tenant_id = (select ferrum_tenant()) or (select ferrum_is_system()));
