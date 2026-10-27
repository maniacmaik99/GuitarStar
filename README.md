# GuitarStar

**Setup-, Mod- und CDLC-Manager für Rocksmith® 2014 Edition – Remastered (Steam; Windows, Linux/Steam Deck als Vorschau).**

Rocksmith 2014 läuft auf aktueller Hardware oft nur mit Bastelei: Audio-Interfaces werden nicht unterstützt, das Rocksmith-Kabel hat spürbare Latenz, und mit Windows-Skalierung passt das Bild nicht auf den Monitor. GuitarStar richtet das mit ein paar Klicks ein – und kann jede Änderung wieder zurücknehmen.

> *English:* GuitarStar is a desktop app that sets up Rocksmith 2014 Remastered for modern hardware: audio interfaces via RS_ASIO, a display scaling fix, custom songs (CDLC) with a built-in CustomsForge browser, optional RSMods with Fast Load, and automatic app updates. Every change to the game folder is backed up and can be undone. The app is available in 11 languages: English, German, Chinese (Simplified), Hindi, Spanish, French, Arabic, Bengali, Portuguese, Russian and Urdu (it follows your system language). Windows is the main platform; Linux/Steam Deck builds are a preview.

## Funktionen

- **Audio-Interfaces statt Rocksmith-Kabel** über [RS_ASIO](https://github.com/mdias/rs_asio): erkennt installierte ASIO-Treiber, bringt fertige Profile für viele Geräte mit (Focusrite Scarlett, Audient, Steinberg, Universal Audio, BOSS, …) und lässt eigene Profile speichern.
- **Tonausgabe wählbar:** über das Interface (niedrigste Latenz), über Windows (z. B. Bluetooth-Kopfhörer) oder Abfrage bei jedem Spielstart.
- **Skalierungs-Fix:** Rocksmith kennt keine Windows-Skalierung; bei 125 % oder mehr ragt das Bild über den Monitor. GuitarStar bringt eine eigene, winzige `dinput8.dll` mit, die das Spiel DPI-aware macht – im Fenster, im Vollbild und im exklusiven Vollbild.
- **Kein Minimieren bei Fokusverlust** (optional): Im Vollbild-Fenster bleibt das Spiel sichtbar, wenn du auf einen zweiten Monitor klickst.
- **Eigene Songs (CDLC):** CustomsForge im eingebauten Fenster – heruntergeladene Songs werden automatisch installiert. Dazu Drag & Drop für `.psarc`/`.zip`, Suche, Aktivieren/Deaktivieren und Löschen. GuitarStar arbeitet nur im Unterordner `dlc\GuitarStar`; die offiziellen Songs bleiben unberührt. Der CDLC-Enabler von CustomsForge lässt sich direkt aus der App installieren.
  Die Werbung auf CustomsForge wird von CustomsForge geschaltet und finanziert die Seite mit; sie lässt sich unter „Einstellungen“ ausblenden (standardmäßig aus).
- **RSMods (optional):** Installation des offiziellen RSMods-Installers mit vorheriger Sicherung; darüber u. a. **Fast Load**, das die Intro-Logos beim Start überspringt. GuitarStar zeigt, ob Fast Load angewendet ist, und kann RSMods mit einem Klick wieder vollständig entfernen (inklusive originaler `cache.psarc`).
- **Discord & Streaming:** Bei Tonausgabe über Windows läuft Rocksmith im geteilten Modus, sodass Discord parallel funktioniert und den Spielton übertragen kann. Der Audio-Tab zeigt das Windows-Ausgabegerät und warnt bei Bluetooth.
- **Spieleinstellungen:** Auflösung, Anzeigemodus, Latenzpuffer, exklusiver Audiomodus.
- **Komponenten mit Updates und Rollback:** Versionen kommen direkt von den offiziellen GitHub-Releases (mit Prüfsummen-Check), jede installierte Version bleibt offline verfügbar.
- **Songliste:** Sortieren, Mehrfachauswahl (aktivieren/deaktivieren/löschen), Erkennung mehrerer Versionen desselben Songs und ein Link zur Song-Seite auf CustomsForge.
- **Song-Updates:** Für Songs, die über das CustomsForge-Fenster geladen wurden, prüft GuitarStar einmal täglich beim Start (manuell bis zu 3-mal am Tag), ob es eine neuere Version gibt. Die Prüfung läuft mit deinem eigenen CustomsForge-Login im eingebetteten Fenster, Song für Song mit Pause; ohne gestartete App wird nichts abgefragt.
- **Verlauf:** Vor jeder Änderung werden die betroffenen Dateien gesichert. Jede Änderung lässt sich zurücknehmen, auch komplett bis zum Originalzustand.
- **Spielstand-Sicherung:** Dein Rocksmith-Profil (Steam-Ordner `userdata/…/221680/remote`) wird auf Knopfdruck, einmal täglich und vor riskanten Aktionen (RSMods, CDLC-Enabler, Originalzustand) gesichert und lässt sich wiederherstellen.
- **Automatische Updates:** GuitarStar meldet neue Versionen beim Start und aktualisiert sich per Klick; unter „Einstellungen“ stehen die installierte Version und ein Knopf zum manuellen Prüfen. Updates sind signiert.
- **11 Sprachen:** Deutsch, Englisch, Chinesisch (vereinfacht), Hindi, Spanisch, Französisch, Arabisch, Bengalisch, Portugiesisch, Russisch und Urdu (Arabisch und Urdu von rechts nach links). Die App folgt der Systemsprache (sonst Englisch) und lässt sich in den Einstellungen umschalten; auch Fehlermeldungen und der Verlauf erscheinen in der gewählten Sprache.

### Geplant

- Song-Infos in der Songliste (Stimmung, Arrangements, Länge)
- Song-Updates über eine offizielle Schnittstelle von CustomsForge, falls das erlaubt wird (siehe [docs/customsforge-update-check.md](docs/customsforge-update-check.md))
- Linux / Steam Deck auf echter Hardware testen und ausbauen (siehe unten)

### Linux / Steam Deck (Vorschau)

Ab Version 0.3.0 gibt es GuitarStar auch als AppImage und `.deb`. Rocksmith läuft unter Linux über Proton; GuitarStar
- findet das Spiel in den Steam-Bibliotheken (auch Flatpak-Steam) und den Proton-Prefix (`compatdata/221680`),
- erkennt, ob [wineasio](https://github.com/mdias/rs_asio/tree/master/docs/linux) im Prefix als 32-Bit-ASIO-Treiber registriert ist, und bietet ihn im Audio-Tab an,
- zeigt die passenden Steam-Startoptionen zum Kopieren, damit Wine die Mod-DLLs lädt, z. B. `WINEDLLOVERRIDES="avrt=n,b;D3DX9_42=n,b" %command%`, und prüft (nur lesend), ob sie in Steam schon eingetragen sind.

wineasio selbst und die Startoptionen richtest du weiterhin selbst ein (Steam → Rocksmith → Eigenschaften → Startoptionen). Noch nicht auf echter Linux-Hardware bzw. einem Steam Deck getestet: Bundles und Selbst-Update, das eingebettete CustomsForge-Fenster, der RSMods-Installer (Windows-Programm) und der Skalierungs-Fix (unter Wine wirkungslos). Rückmeldungen dazu sind sehr willkommen.

## Installation

**[Neueste Version herunterladen](https://github.com/maniacmaik99/GuitarStar/releases/latest)** – unter „Assets“:

| System | Datei |
|---|---|
| Windows | `GuitarStar_…_x64-setup.exe` herunterladen und starten |
| Linux / Steam Deck (Vorschau) | `GuitarStar_…_amd64.AppImage` (ausführbar machen und starten) oder `GuitarStar_…_amd64.deb` |

Rust, Node.js o. Ä. werden dafür nicht gebraucht.

Das Windows-Setup ist (noch) nicht signiert, deshalb warnt Windows SmartScreen beim ersten Start: **Weitere Informationen → Trotzdem ausführen**.

Ab Version 0.2.0 meldet GuitarStar neue Versionen selbst und aktualisiert sich per Klick. Updates sind signiert; die App installiert nur Updates mit gültiger Signatur. Wer noch 0.1.0 hat, installiert die neueste Version einmal von Hand.

Voraussetzungen zum Spielen:
- Rocksmith 2014 Edition – Remastered in der **Steam-Version**
- für Audio-Interfaces: der ASIO-Treiber des Herstellers (z. B. „Focusrite Control“) – Rocksmith braucht die **32-Bit**-Variante, die die meisten Treiber mitinstallieren
- Rocksmith einmal starten, damit es seine `Rocksmith.ini` anlegt

## Selbst bauen

Benötigt: [Rust](https://rustup.rs) (stable), [Node.js](https://nodejs.org) 20+, unter Windows WebView2 (in Windows 11 enthalten), unter Linux die [Tauri-Systempakete](https://tauri.app/start/prerequisites/#linux) (Debian/Ubuntu: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf build-essential libssl-dev libxdo-dev`).

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

### Neue Version veröffentlichen

1. Version in `package.json`, `src-tauri/Cargo.toml` und `src-tauri/tauri.conf.json` erhöhen.
2. Committen, dann Tag pushen: `git tag v0.3.0 && git push origin v0.3.0`.
3. Der Workflow „Release“ baut das Setup, signiert das Update und veröffentlicht beides.

Einmalig nötig: der Signaturschlüssel als GitHub-Secret (`scripts/set-release-secrets.ps1`). Der Schlüssel liegt unter `~/.tauri/guitarstar.key` und muss gesichert werden – ohne ihn können installierte Versionen keine Updates mehr bekommen.

## Aufbau

| Ordner | Inhalt |
|---|---|
| `src/` | Oberfläche (SvelteKit, TypeScript); Texte pro Sprache in `src/lib/i18n/<sprache>.ts` (Deutsch ist die Referenz) |
| `src-tauri/src/` | Tauri-App: Befehle zwischen Oberfläche und Kernlogik |
| `src-tauri/core/` | Kernlogik ohne UI: Spielordner, INI-Dateien, Audio-Profile, Komponenten, Sicherungen |
| `src-tauri/core/profiles/` | Geräteprofile (`devices.json`) und RS_ASIO-Vorlage |
| `native/dpi-fix/` | Quellcode der `dinput8.dll` (Skalierungs-Fix, 32-Bit, `no_std`) |

### Ein Gerät hinzufügen

Profile stehen in [`src-tauri/core/profiles/devices.json`](src-tauri/core/profiles/devices.json). Wenn dein Interface fehlt: in der App unter „Audio“ einrichten, als eigenes Profil speichern und die Werte gern als Pull Request oder Issue einreichen.

## Wie GuitarStar mit dem Spiel umgeht

- Am Spiel selbst wird nichts verändert; die `Rocksmith2014.exe` und ihr Kopierschutz bleiben unberührt.
- Komponenten werden als zusätzliche DLLs in den Spielordner gelegt (`avrt.dll` für RS_ASIO, `dinput8.dll` für den Skalierungs-Fix) und über Konfigurationsdateien eingestellt. Ausnahme: RSMods‘ Fast Load verändert die Spieldatei `cache.psarc`; GuitarStar sichert vorher das Original und kann es wiederherstellen.
- Alles, was GuitarStar im Spielordner anlegt oder ändert, wird vorher gesichert und kann über „Verlauf“ zurückgenommen werden.

## Danksagung

- [RS_ASIO](https://github.com/mdias/rs_asio) von mdias (MIT) – die Grundlage für Audio-Interfaces; die Geräteprofile stammen aus dessen Dokumentation.
- Die Rocksmith-Community, insbesondere [CustomsForge](https://customsforge.com) und [RSMods](https://github.com/Lovrom8/RSMods).

## Rechtliches

GuitarStar ist ein inoffizielles Community-Projekt und steht in keiner Verbindung zu Ubisoft. Rocksmith® ist eine Marke von Ubisoft Entertainment. Die Nutzung erfolgt auf eigene Verantwortung.

Lizenz: [MIT](LICENSE)
