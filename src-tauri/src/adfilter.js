// Injected into the CustomsForge view only when the user turned on
// "Werbung ausblenden". Hides ad slots; it does not touch the rest of the page.
(() => {
  if (window.__guitarstarAdFilter) return;
  window.__guitarstarAdFilter = true;

  // Ad slots on ignition4.customsforge.com (Playwire RAMP) and common ad markup.
  const css = [
    '[class^="samsad-"]',
    '[class*=" samsad-"]',
    ".footer-ad",
    'div:has(> [class^="samsad-"])',
    "ins.adsbygoogle",
    '[id^="google_ads_"]',
    '[id^="div-gpt-ad"]',
    '[id^="pw-"]',
    '[class^="pw-"]',
    '[class*=" pw-"]',
    '[id*="bottom_rail"]',
    '[id*="corner_ad"]',
  ].join(",") + "{display:none!important}";

  const addStyle = () => {
    if (document.getElementById("guitarstar-adfilter")) return;
    const style = document.createElement("style");
    style.id = "guitarstar-adfilter";
    style.textContent = css;
    (document.head || document.documentElement).appendChild(style);
  };

  const AD = /doubleclick|googlesyndication|google_ads_iframe|amazon-adsystem|intergient|playwire|adnxs|criteo|pubmatic|rubiconproject|openx|casalemedia|taboola|outbrain/i;

  // Ad iframes are often injected later and wrapped in a fixed banner; hide the
  // iframe, and its fixed/sticky wrapper if there is one close by.
  const hide = (frame) => {
    let target = frame;
    let el = frame;
    for (let i = 0; i < 4 && el.parentElement && el.parentElement !== document.body; i++) {
      el = el.parentElement;
      const pos = getComputedStyle(el).position;
      if (pos === "fixed" || pos === "sticky") {
        target = el;
        break;
      }
    }
    target.style.setProperty("display", "none", "important");
  };

  let queued = false;
  const sweep = () => {
    queued = false;
    addStyle();
    for (const f of document.querySelectorAll("iframe")) {
      if (AD.test(f.src || "") || AD.test(f.id || "") || AD.test(f.name || "")) hide(f);
    }
  };
  const schedule = () => {
    if (!queued) {
      queued = true;
      requestAnimationFrame(sweep);
    }
  };

  const start = () => {
    addStyle();
    sweep();
    new MutationObserver(schedule).observe(document.documentElement, { childList: true, subtree: true });
  };
  if (document.documentElement) start();
  else document.addEventListener("DOMContentLoaded", start);
})();
