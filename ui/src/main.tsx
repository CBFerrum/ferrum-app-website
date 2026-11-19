// The Website's own interface at /website/: the pages and posts, published and drafts, each with
// its preview, and every record in the records frame's lists and forms.

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { createBrowserRouter, RouterProvider } from "react-router";
import { addCatalogs, routeBase, wordsReady } from "@ferrum/client";
import { ConfirmHost, Toaster } from "@ferrum/kit";
import { setFrameMode } from "@ferrum/records/frame";
import { routes } from "./routes";
import "./styles.css";

// Its own words in the page's language (src/locales), fetched as the page starts; it is drawn once they are in.
void addCatalogs({ ar: () => import("./locales/ar.json"), hi: () => import("./locales/hi.json") });

const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false, refetchOnWindowFocus: false, staleTime: 5_000 } } });

setFrameMode("app", ["website"]);

const router = createBrowserRouter(routes, { basename: routeBase("website") });

// Drawn once the page's words are in (its language's catalogs, fetched as the page starts).
void wordsReady().then(() =>
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
        <Toaster />
        <ConfirmHost />
      </QueryClientProvider>
    </StrictMode>,
  ),
);
