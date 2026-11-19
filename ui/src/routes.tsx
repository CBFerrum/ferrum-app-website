// The Website's route table, apart from the page that mounts it so a test can mount the real thing.

import { useEffect } from "react";
import { Link, type RouteObject, useRouteError } from "react-router";
import { shell, t } from "@ferrum/client";
import { ListSkeleton, StatePanel } from "@ferrum/kit";
import { RequireAuth } from "@ferrum/records/frame";

const page = (name: "ListView" | "ReportView") => () => import("./records").then((m) => ({ Component: m[name] }));

function RouteError() {
  return <StatePanel kind="failed" error={useRouteError()} onRetry={() => window.location.reload()} />;
}

function NotFound() {
  return (
    <StatePanel kind="empty" title={t("Page not found")}>
      <Link to="/">{t("Go to the website")}</Link>
    </StatePanel>
  );
}

function Started() {
  useEffect(() => shell().ready(), []);
  return null;
}

export const routes: RouteObject[] = [
  {
    path: "/",
    element: (
      <>
        <Started />
        <RequireAuth />
      </>
    ),
    errorElement: <RouteError />,
    hydrateFallbackElement: <ListSkeleton />,
    children: [
      { index: true, lazy: () => import("./Home").then((m) => ({ Component: m.Home })) },
      { path: "e/:entity", lazy: page("ListView") },
      // A web page's own editor; any other record, and a new page, the records frame's form.
      { path: "e/:entity/:id", lazy: () => import("./BlockEditor").then((m) => ({ Component: m.Record })) },
      { path: "r/:name", lazy: page("ReportView") },
      { path: "*", element: <NotFound /> },
    ],
  },
];
