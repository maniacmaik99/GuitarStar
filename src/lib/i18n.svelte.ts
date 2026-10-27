// Minimal i18n. German (`i18n/de.ts`) is the reference dictionary; every
// language lives in its own file under `i18n/`. Missing keys fall back to
// English, then German.
import { invoke } from "@tauri-apps/api/core";
import { de, type DeepPartial, type Dict } from "./i18n/de";
import { en } from "./i18n/en";
import { zh } from "./i18n/zh";
import { hi } from "./i18n/hi";
import { es } from "./i18n/es";
import { fr } from "./i18n/fr";
import { ar } from "./i18n/ar";
import { bn } from "./i18n/bn";
import { pt } from "./i18n/pt";
import { ru } from "./i18n/ru";
import { ur } from "./i18n/ur";

/** German, English and the most widely spoken languages. */
export const LANGUAGES = [
  { code: "de", name: "Deutsch", locale: "de-DE", rtl: false },
  { code: "en", name: "English", locale: "en-US", rtl: false },
  { code: "zh", name: "中文（简体）", locale: "zh-CN", rtl: false },
  { code: "hi", name: "हिन्दी", locale: "hi-IN", rtl: false },
  { code: "es", name: "Español", locale: "es-ES", rtl: false },
  { code: "fr", name: "Français", locale: "fr-FR", rtl: false },
  { code: "ar", name: "العربية", locale: "ar", rtl: true },
  { code: "bn", name: "বাংলা", locale: "bn-BD", rtl: false },
  { code: "pt", name: "Português", locale: "pt-BR", rtl: false },
  { code: "ru", name: "Русский", locale: "ru-RU", rtl: false },
  { code: "ur", name: "اردو", locale: "ur-PK", rtl: true },
] as const;

export type Lang = (typeof LANGUAGES)[number]["code"];

const STORAGE_KEY = "guitarstar.lang";

type Leaves<T> = {
  [K in keyof T & string]: T[K] extends string ? K : `${K}.${Leaves<T[K]>}`;
}[keyof T & string];

/** Every valid `t()` key, e.g. "game.resolution". */
export type MessageKey = Leaves<Dict>;

const dictionaries: Record<Lang, DeepPartial<Dict>> = { de, en, zh, hi, es, fr, ar, bn, pt, ru, ur };

const isLang = (code: string | null | undefined): code is Lang => LANGUAGES.some((l) => l.code === code);
const info = (lang: Lang) => LANGUAGES.find((l) => l.code === lang) ?? LANGUAGES[1];

/** Saved choice, else the system language if supported, else English. */
function initialLang(): Lang {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (isLang(saved)) return saved;
  } catch {
    // Storage unavailable: fall through to the system language.
  }
  const system = typeof navigator !== "undefined" ? navigator.language?.toLowerCase().split("-")[0] : undefined;
  return isLang(system) ? system : "en";
}

const current = $state<{ lang: Lang }>({ lang: initialLang() });

function applyHtmlLang() {
  if (typeof document === "undefined") return;
  document.documentElement.lang = current.lang;
  document.documentElement.dir = info(current.lang).rtl ? "rtl" : "ltr";
}
applyHtmlLang();

let backendSync: Promise<void> = Promise.resolve();

/** Tells the backend the language, so its messages (errors, history labels …) use it too. */
function syncBackend() {
  try {
    backendSync = invoke("set_language", { lang: current.lang }).then(
      () => {},
      () => {},
    );
  } catch {
    // Not running inside Tauri (plain web build).
  }
}
syncBackend();

/** Resolves once the backend knows the current language. */
export function languageSynced(): Promise<void> {
  return backendSync;
}

export function getLang(): Lang {
  return current.lang;
}

export function setLang(lang: Lang) {
  current.lang = lang;
  applyHtmlLang();
  syncBackend();
  try {
    localStorage.setItem(STORAGE_KEY, lang);
  } catch {
    // Not persisted; the choice still applies for this session.
  }
}

function lookup(dict: DeepPartial<Dict>, key: string): string | undefined {
  let node: unknown = dict;
  for (const part of key.split(".")) {
    if (typeof node !== "object" || node === null) return undefined;
    node = (node as Record<string, unknown>)[part];
  }
  return typeof node === "string" ? node : undefined;
}

/** Translates `key` into the current language; `{name}` placeholders are filled from `params`. */
export function t(key: MessageKey, params?: Record<string, string | number>): string {
  const text = lookup(dictionaries[current.lang], key) ?? lookup(en, key) ?? lookup(de, key) ?? key;
  if (!params) return text;
  return text.replace(/\{(\w+)\}/g, (match, name: string) => (name in params ? String(params[name]) : match));
}

/**
 * A label stored by the backend: translated from its key when the dictionary
 * knows it, otherwise the text stored with it (unknown keys).
 */
export function storedLabel(
  key: string | null | undefined,
  params: Record<string, string> | null | undefined,
  fallback: string,
): string {
  if (!key || lookup(de, key) === undefined) return fallback;
  return t(key as MessageKey, params ?? undefined);
}

/** BCP 47 locale for dates and numbers in the current language. */
export function locale(): string {
  return info(current.lang).locale;
}

export function formatNumber(value: number, options?: Intl.NumberFormatOptions): string {
  return value.toLocaleString(locale(), options);
}
