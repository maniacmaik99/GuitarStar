# GuitarStar

**Setup-, Mod- und CDLC-Manager für Rocksmith® 2014 Edition – Remastered (Steam, Windows).**

Rocksmith 2014 läuft auf aktueller Hardware oft nur mit Bastelei: Audio-Interfaces werden nicht unterstützt, das Rocksmith-Kabel hat spürbare Latenz, und mit Windows-Skalierung passt das Bild nicht auf den Monitor. GuitarStar richtet das mit ein paar Klicks ein – und kann jede Änderung wieder zurücknehmen.

> *English:* GuitarStar is a desktop app that sets up Rocksmith 2014 Remastered for modern hardware: audio interfaces via RS_ASIO, display scaling, and (soon) custom songs. Every change to the game folder is backed up and can be undone. The UI is currently German only.

## Funktionen

- **Audio-Interfaces statt Rocksmith-Kabel** über [RS_ASIO](https://github.com/mdias/rs_asio): erkennt installierte ASIO-Treiber, bringt fertige Profile für viele Geräte mit (Focusrite Scarlett, Audient, Steinberg, Universal Audio, BOSS, …) und lässt eigene Profile speichern.
- **Tonausgabe wählbar:** über das Interface (niedrigste Latenz), über Windows (z. B. Bluetooth-Kopfhörer) oder Abfrage bei jedem Spielstart.
- **Skalierungs-Fix:** Rocksmith kennt keine Windows-Skalierung; bei 125 % oder mehr ragt das Bild über den Monitor. GuitarStar bringt eine eigene, winzige `dinput8.dll` mit, die das Spiel DPI-aware macht – im Fenster, im Vollbild und im exklusiven Vollbild.
- **Kein Minimieren bei Fokusverlust** (optional): Im Vollbild-Fenster bleibt das Spiel sichtbar, wenn du auf einen zweiten Monitor klickst.
- **Eigene Songs (CDLC):** CustomsForge im eingebauten Fenster – heruntergeladene Songs werden automatisch installiert. Dazu Drag & Drop für `.psarc`/`.zip`, Suche, Aktivieren/Deaktivieren und Löschen. GuitarStar arbeitet nur im Unterordner `dlc\GuitarStar`; die offiziellen Songs bleiben unberührt. Der CDLC-Enabler von CustomsForge lässt sich direkt aus der App installieren.
  Die Werbung auf CustomsForge wird von CustomsForge geschaltet und finanziert die Seite mit; sie lässt sich unter „Einstellungen“ ausblenden (standardmäßig aus).
- **Spieleinstellungen:** Auflösung, Anzeigemodus, Latenzpuffer, exklusiver Audiomodus.
- **Komponenten mit Updates und Rollback:** Versionen kommen direkt von den offiziellen GitHub-Releases (mit Prüfsummen-Check), jede installierte Version bleibt offline verfügbar.
- **Verlauf:** Vor jeder Änderung werden die betroffenen Dateien gesichert. Jede Änderung lässt sich zurücknehmen, auch komplett bis zum Originalzustand.

### Geplant

- Song-Infos direkt aus den Dateien (Stimmung, Pfade, Länge)
- RSMods als optionale Komponente
- Linux / Steam Deck (Proton, wineasio)

## Installation

Es gibt noch keine fertigen Releases. Bis dahin: selbst bauen (siehe unten).

Voraussetzungen zum Spielen:
- Rocksmith 2014 Edition – Remastered in der **Steam-Version**
- für Audio-Interfaces: der ASIO-Treiber des Herstellers (z. B. „Focusrite Control“) – Rocksmith braucht die **32-Bit**-Variante, die die meisten Treiber mitinstallieren
- Rocksmith einmal starten, damit es seine `Rocksmith.ini` anlegt

## Selbst bauen

Benötigt: [Rust](https://rustup.rs) (stable), [Node.js](https://nodejs.org) 20+, unter Windows WebView2 (in Windows 11 enthalten).

```bash
npm install
npm run tauri dev      # Entwicklungsmodus
npm run tauri build    # Installer bauen
```

Tests der Kernlogik:

```bash
cd src-tauri
cargo test -p guitarstar_core
```

Die mitgelieferte `dinput8.dll` liegt fertig gebaut unter `src-tauri/core/assets/`. Neu bauen (32-Bit, ohne Visual Studio):

```bash
rustup target add i686-pc-windows-msvc
powershell -File native/dpi-fix/build.ps1
```

## Aufbau

| Ordner | Inhalt |
|---|---|
| `src/` | Oberfläche (SvelteKit, TypeScript) |
| `src-tauri/src/` | Tauri-App: Befehle zwischen Oberfläche und Kernlogik |
| `src-tauri/core/` | Kernlogik ohne UI: Spielordner, INI-Dateien, Audio-Profile, Komponenten, Sicherungen |
| `src-tauri/core/profiles/` | Geräteprofile (`devices.json`) und RS_ASIO-Vorlage |
| `native/dpi-fix/` | Quellcode der `dinput8.dll` (Skalierungs-Fix, 32-Bit, `no_std`) |

### Ein Gerät hinzufügen

Profile stehen in [`src-tauri/core/profiles/devices.json`](src-tauri/core/profiles/devices.json). Wenn dein Interface fehlt: in der App unter „Audio“ einrichten, als eigenes Profil speichern und die Werte gern als Pull Request oder Issue einreichen.

## Wie GuitarStar mit dem Spiel umgeht

- Am Spiel selbst wird nichts verändert; die `Rocksmith2014.exe` und ihr Kopierschutz bleiben unberührt.
- Komponenten werden als zusätzliche DLLs in den Spielordner gelegt (`avrt.dll` für RS_ASIO, `dinput8.dll` für den Skalierungs-Fix) und über Konfigurationsdateien eingestellt.
- Alles, was GuitarStar im Spielordner anlegt oder ändert, wird vorher gesichert und kann über „Verlauf“ zurückgenommen werden.

## Danksagung

- [RS_ASIO](https://github.com/mdias/rs_asio) von mdias (MIT) – die Grundlage für Audio-Interfaces; die Geräteprofile stammen aus dessen Dokumentation.
- Die Rocksmith-Community, insbesondere [CustomsForge](https://customsforge.com) und [RSMods](https://github.com/Lovrom8/RSMods).

## Rechtliches

GuitarStar ist ein inoffizielles Community-Projekt und steht in keiner Verbindung zu Ubisoft. Rocksmith® ist eine Marke von Ubisoft Entertainment. Die Nutzung erfolgt auf eigene Verantwortung.

Lizenz: [MIT](LICENSE)
