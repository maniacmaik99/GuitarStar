# Detecting newer versions of installed songs on CustomsForge

Status: option B is implemented (since 0.4.0), with one change: the page
reports through its document title (`GSCHK:{…}`, read by
`on_document_title_changed`) instead of a remote IPC capability, so the
CustomsForge page still gets no access to any app command. Limits: one
automatic run per day at app start, at most 3 manual runs per day, at most 50
songs per run, 2.5 s between songs, stop on login redirect/401/403/429.
Code: `src-tauri/src/updatecheck.js`, `src-tauri/core/src/versions.rs`,
`UpdateCheck` in `src-tauri/src/commands.rs`. The HTML selectors (download
button text "… vN", "Last revised" row) still need a check with a logged-in
account. Option A (asking CustomsForge) remains the preferred long-term route.

Research: investigated on 2026-10-27 without logging in, using only public
resources (a dozen requests in total: the home page, five public script
bundles, `robots.txt`, `sitemap.xml`, `/tools/cfsm2`, one song page and one
API path).

## Goal

On app start (or when the Songs tab opens), tell the user that a newer version
of an installed custom song exists on CustomsForge. Song data on CustomsForge
is only visible to logged-in users, and the login lives in the cookies of the
embedded CustomsForge webview.

What GuitarStar already knows per file (since the "song sources" feature):
`cdlc-sources.json` maps a file name to the CustomsForge id of the song it was
downloaded from (`cf_id`), the download time and the host that delivered the
file. Files installed by drag & drop or before that feature have no id; for
those only the file name (artist, title, `_vN`) is available. The songs' own
metadata is encrypted and is deliberately not read.

## Findings

### Site architecture

- `ignition4.customsforge.com` is a Laravel application with **Livewire 3**
  (server-rendered components, updates via `POST /livewire-<hash>/update` with
  a signed component snapshot and the CSRF token from
  `<meta name="csrf-token">`) and Alpine.js. There is no public JSON API for
  song data in the bundles; song details are rendered server-side.
- Relevant Livewire components embedded in every page (from `wire:snapshot`):
  - `modal.cdlc-modal` — the song detail modal. Its public state contains
    `cdlcId`, `cdlcIds`, `cdlc`, `isCollected`, `isSaved`, `isLoved`,
    `loveCount`, … It is opened with the browser event
    `Livewire.dispatch('openCdlcModal', { cdlcId, cdlcIds })`
    (`app-*.js`). The song data itself (`cdlc`) is only filled after the
    server round trip, i.e. only with a session.
  - `modal.report-cdlc-modal`, `modal.adopt-cdlc-modal`,
    `modal.delete-cdlc-modal`, `navbar`, hover cards for artist/album/user.
- Song pages: `GET /cdlc/<id>` redirects anonymous users to `/login`
  (with `Accept: application/json` it answers `401`). Download links have the
  form `/cdlc/<id>/download?expires=…&platform=pc&signature=…` (Laravel signed
  URLs, short-lived).

### Endpoints seen in the public JavaScript

| Endpoint | Method | Purpose (inferred) | Login |
| --- | --- | --- | --- |
| `/api/savedcdlc/toggle/<id>` | `PATCH` | "save for later" toggle; JSON `{ toastText }` | yes (`405` for `GET`, CSRF needed) |
| collection "add" (URL taken from the button's `href`) | `PATCH` | adds a song to the user's collection; JSON `{ toastText, downloadLink?, refreshPage?, redirectUrl? }`, then dispatches `cdlc-collection-changed` | yes |
| `/livewire-<hash>/update` | `POST` | all Livewire component calls (song modal, search, …) | session + CSRF; snapshots are signed |
| `/csrf/refresh` | `GET` | refreshes the CSRF token | session |
| `/users/mentions?q=`, `/gif/search?q=`, `/gif/featured`, `/gif/categories` | `GET` | comment editor helpers | yes |
| `/analytics/landing/cta` | `POST` (beacon) | landing page tracking | no |
| `/broadcasting/auth`, Pusher / Mercure (`/.well-known/mercure`) | | live notifications | yes |

`robots.txt` disallows `/api/`, `/livewire`, `/auth/`, `/tools/download/` and
`/eof/download/` for crawlers.

Nothing in the public bundles reads a song's version, its "updated at" date,
or a "changed since you downloaded it" flag through a JSON endpoint. The only
`version`/`outdated` fields in the bundles belong to the playthrough-video
widget (`isOutdated` = video recorded for an older song version) and to the
upload form, which parses a creator's own file (`rs2dlcParsed`).

### CFSM 2

The `/tools/cfsm2` page says CustomsForge Song Manager 2 "syncs [your CDLC
folder] against your Ignition collection" and shows a "song updates banner".
So CustomsForge itself has a server-side notion of "a newer version of a song
in your collection exists", exposed to their own desktop tool through an
interface that is not documented publicly. Also, the "collection" seems to be
filled automatically when a user downloads a song (the collection "add"
response can carry the `downloadLink`).

## Options

### A. Ask CustomsForge (preferred long-term)

Ask the CustomsForge team (Discord) whether the interface CFSM 2 uses for
"collection sync / song updates" may be used by GuitarStar, ideally with a
per-user token the user creates on the site. That would be stable, within the
rules, and could replace everything below. Until then, B is the least invasive
way.

### B. Report from inside the embedded view (proposed design)

Keep the page sandboxed and let it only *report* what the user's own session
can see.

1. **A second, narrow capability** `src-tauri/capabilities/customsforge.json`:

   ```json
   {
     "identifier": "customsforge-report",
     "description": "Lets the CustomsForge view report song versions, nothing else",
     "webviews": ["customsforge"],
     "remote": { "urls": ["https://ignition4.customsforge.com/*"] },
     "permissions": ["allow-cdlc-report-versions"]
   }
   ```

   with an app permission (`src-tauri/permissions/cdlc-report.toml`) that
   allows exactly one command, `cdlc_report_versions`. The main capability
   stays bound to the `main` webview; the CustomsForge view gets no file,
   dialog, opener or shell access. Pages from any other host (Google Drive,
   Mega, ads in iframes) get nothing, because `remote.urls` only matches
   CustomsForge.

2. **The command only stores data**:
   `cdlc_report_versions(entries: Vec<{ cf_id: String, version: Option<String>, updated_at: Option<String> }>)`
   validates every field (id charset as in `sources::valid_id`, length limits,
   at most a few hundred entries per call), keeps only ids that are in
   `cdlc-sources.json`, writes `cdlc-versions.json` atomically and emits an
   event to the main window. It never downloads, opens or deletes anything.

3. **An initialization script** (like `adfilter.js`, only active on
   `ignition4.customsforge.com`, only when the user is logged in) runs when
   the main window asks for a check (e.g. via `eval` of a small trigger
   function when the Songs tab opens and the view exists):
   - gets the list of ids to check from the app (passed in the trigger call,
     not fetched from the page),
   - for each id at most once per day: `fetch('/cdlc/<id>', { credentials:
     'same-origin' })` and reads the version and the last-update date from
     the returned HTML (`DOMParser`, selectors to be determined with a
     logged-in session), or – if CustomsForge exposes it – reads the
     "updated since collected" marker from the user's collection page in one
     request instead of one per song,
   - sends the result with
     `window.__TAURI_INTERNALS__.invoke('cdlc_report_versions', { entries })`.
   Requests are sequential with a delay (e.g. 2–5 s), stop on the first
   `401`/`403`/`429`, and run in the background view without the user having
   to open it.

4. **Comparison in the core crate**: compare the reported version with the
   `_vN` of the installed file name (`cdlc::parse_name`) and, if the site
   reports an update date, with `downloaded_at` from `cdlc-sources.json`.
   Show "Update available" with a link (`customsforge_open_url`) in the song
   list. Downloading the new version stays a click by the user.

For songs without a stored `cf_id`, matching by artist/title through the
site's search would be possible but much less reliable and causes more
requests; leave it out at first.

### C. Alternatives considered

- **Reading the cookies in Rust and calling the site from `reqwest`**: would
  move the user's session out of the webview, is fragile (WebView2 cookie
  store, HttpOnly) and looks like a bot to the site. Not recommended.
- **Driving the Livewire `cdlc-modal` component** (`openCdlcModal`) and
  reading its state: depends on internal component names and signed
  snapshots; brittle, and it opens UI. Not recommended.
- **No automation, only a link**: the "on CustomsForge" link in the song list
  (already implemented) lets the user check a song by hand. Zero risk.

## Risks and rules

- **Site changes**: HTML selectors, routes (`/cdlc/<id>`) and the Livewire
  path hash (`/livewire-97261508/`) can change with any deployment. The check
  must fail quietly ("check not possible") and never block other features.
- **Terms of use / fairness**: only the user's own session, only pages a user
  can open anyway, no bypassing of login, captcha or rate limits, no
  downloads without a click. `robots.txt` disallows `/api/` and `/livewire`
  for crawlers; the design therefore uses the ordinary song pages (or the
  collection page) and no hidden API. Ask CustomsForge before shipping
  (option A).
- **Rate limits**: at most one request per installed song per day, sequential
  with a delay, a hard cap per run (e.g. 50 songs), stop on `429`; remember the
  last check per id in `cdlc-versions.json`.
- **Ads and privacy**: the requests run inside the CustomsForge page and carry
  its cookies; nothing is sent anywhere else. The reported data stays local.
- **Security**: the IPC bridge in a remote page is the main risk. Keep the
  capability to one validating, store-only command; never grant it to
  file hosts or other domains; treat all reported values as untrusted text.

## Needs a manual test with a logged-in account

- What `/cdlc/<id>` shows when logged in: where version and last update date
  are, and whether they are stable selectors.
- Whether the collection page marks songs that changed since they were
  collected/downloaded (that would allow a single request for all songs).
- Whether downloads add songs to the collection automatically.
- Whether `remote.urls` capabilities work for the child webview created with
  `WebviewBuilder` (label `customsforge`) on Windows (WebView2) and Linux
  (WebKitGTK).
