// Injected into the CustomsForge view. When GuitarStar asks for it
// (window.__gsCheckSongs), it opens the given song pages with the user's own
// login, one at a time, and reads the version and the "Last revised" date.
// Results go back through the page title, which the app watches: the page
// never gets access to the app itself.
(() => {
  if (location.hostname !== "ignition4.customsforge.com" || window.__gsCheckSongs) return;

  const PREFIX = "GSCHK:";
  const DELAY_MS = 2500;
  let running = false;

  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const text = (el) => (el?.textContent || "").replace(/\s+/g, " ").trim();

  // Each message is shown in the title briefly, then the title is restored.
  const report = (message) =>
    new Promise((resolve) => {
      const previous = document.title;
      document.title = PREFIX + JSON.stringify(message);
      setTimeout(() => {
        document.title = previous;
        resolve();
      }, 200);
    });

  function parse(html) {
    const doc = new DOMParser().parseFromString(html, "text/html");
    let version = null;
    for (const el of doc.querySelectorAll("a, button")) {
      const t = text(el);
      const m = /download/i.test(t) && t.match(/\bv(\d+(?:\.\d+)*)\b/i);
      if (m) {
        version = m[1];
        break;
      }
    }
    let revised = null;
    for (const el of doc.querySelectorAll("body *")) {
      const t = text(el);
      if (t.length > 60) continue;
      const m = t.match(/^last revised\s*(.*)$/i);
      if (m && m[1]) {
        revised = m[1].trim();
        break;
      }
    }
    return { version, revised };
  }

  window.__gsCheckSongs = async (ids) => {
    if (running || !Array.isArray(ids)) return;
    running = true;
    try {
      for (let i = 0; i < ids.length; i++) {
        if (i) await sleep(DELAY_MS);
        let res;
        try {
          res = await fetch(`/cdlc/${encodeURIComponent(ids[i])}`, { credentials: "same-origin" });
        } catch {
          await report({ type: "error" });
          return;
        }
        if (res.status === 401 || res.status === 403 || /^\/login\b/.test(new URL(res.url).pathname)) {
          await report({ type: "login" });
          return;
        }
        if (res.status === 429) {
          await report({ type: "limited" });
          return;
        }
        if (!res.ok) {
          await report({ type: "song", id: ids[i], ok: false });
          continue;
        }
        const { version, revised } = parse(await res.text());
        await report({ type: "song", id: ids[i], ok: true, version, revised });
      }
      await report({ type: "done" });
    } finally {
      running = false;
    }
  };
})();
