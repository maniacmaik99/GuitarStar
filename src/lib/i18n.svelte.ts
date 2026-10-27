// Minimal i18n: German is the reference dictionary, English may omit keys (falls back to German).

export type Lang = "de" | "en";

const STORAGE_KEY = "guitarstar.lang";

const de = {
  common: {
    save: "Speichern",
    saving: "Wird gespeichert …",
    recheck: "Neu prüfen",
    remove: "Entfernen",
    install: "Installieren",
    installed: "installiert",
    notInstalled: "nicht installiert",
    close: "schließen",
    reload: "Neu laden",
    installerLoading: "Installer wird geladen …",
    installerStarted: "Installer gestartet. Danach hier „Neu prüfen“ klicken.",
  },
  app: {
    tabGame: "Spiel",
    tabAudio: "Audio",
    tabSongs: "Songs",
    tabComponents: "Komponenten",
    tabHistory: "Verlauf",
    tabSettings: "Einstellungen",
    chooseFolderTitle: "Rocksmith-2014-Ordner auswählen",
    checkingFolder: "Ordner wird geprüft …",
    change: "ändern",
    loading: "Lade …",
    welcome: "Willkommen bei GuitarStar",
    intro: "Wähle zuerst deinen Rocksmith-2014-Ordner aus. GuitarStar ändert dort nur Dateien, die es vorher gesichert hat.",
    detected: "Gefundene Installationen:",
    noneDetected: "Über Steam wurde keine Installation gefunden.",
    chooseFolder: "Ordner auswählen …",
  },
  update: {
    available: "GuitarStar {version} ist da",
    current: "(installiert: {version})",
    failed: "Update fehlgeschlagen: {error}",
    downloading: "Wird geladen … {progress}%",
    now: "Jetzt aktualisieren",
    later: "später",
  },
  game: {
    keepVisibleOn: "Rocksmith minimiert sich ab dem nächsten Start nicht mehr.",
    keepVisibleOff: "Rocksmith verhält sich wieder wie vorher.",
    dpiRemoving: "Einstellung wird entfernt …",
    dpiRemoved: "Die wirkungslose DPI-Einstellung wurde entfernt.",
    savingSettings: "Einstellungen werden gespeichert …",
    settingsSaved: "Einstellungen gespeichert.",
    installingFix: "Skalierungs-Fix wird installiert …",
    fixInstalledDone: "Skalierungs-Fix installiert. Starte Rocksmith neu.",
    dpiTitle: "DPI-Einstellung entfernen",
    dpiText:
      "Die Windows-Einstellung „Skalierung durch: Anwendung“ ist für Rocksmith gesetzt, wirkt bei der Steam-Version aber nicht. Sie lässt sich gefahrlos entfernen; der Skalierungs-Fix unten ersetzt sie.",
    scalingTitle: "Windows-Skalierung",
    fixActive: "Skalierungs-Fix aktiv",
    fixActiveText:
      "Rocksmith bekommt die echte Auflösung deines Monitors – im Fenster, im Vollbild und im exklusiven Vollbild. Entfernen lässt sich der Fix unter „Komponenten“.",
    updateFix: "Fix auf {version} aktualisieren",
    scaledText:
      "Dein Monitor ist skaliert. Rocksmith kommt damit nicht klar: Windows vergrößert das Spiel, und es ragt über den Bildschirm. Der Skalierungs-Fix behebt das in allen Anzeigemodi.",
    fixConflict: "Achtung: Im Spielordner liegt schon eine andere {files}. Sie wird gesichert und ersetzt.",
    installFix: "Skalierungs-Fix installieren",
    iniMissingTitle: "Rocksmith.ini fehlt noch",
    iniMissingText:
      "Das Spiel legt seine Einstellungsdatei erst beim ersten Start an. Starte Rocksmith einmal kurz über Steam, beende es wieder und lade diese Seite dann neu.",
    checking: "Prüfe …",
    display: "Bild",
    resolution: "Auflösung",
    monitorSuffix: " (Monitor)",
    displayMode: "Anzeigemodus",
    windowed: "Fenster",
    fullscreenWindow: "Vollbild (Fenster)",
    exclusiveFullscreen: "Exklusives Vollbild",
    focusLoss: "Bei Fokusverlust",
    keepVisible: "nicht minimieren (z. B. beim Klick auf den zweiten Monitor)",
    exclusiveHint:
      "Im exklusiven Vollbild minimiert DirectX das Spiel beim Fokusverlust immer – das lässt sich nicht verhindern. Für zwei Monitore „Vollbild (Fenster)“ wählen; dank Skalierungs-Fix sieht es genauso aus.",
    adapt: "Ohne Fix: Auflösung an Windows-Skalierung ({percent} %) anpassen (nur im Fenstermodus)",
    adaptOn:
      "Rocksmith bekommt {w} × {h}; Windows vergrößert das Fenster dann genau auf {tw} × {th}. Das Bild ist dadurch minimal weicher.",
    adaptOff:
      "Ohne Anpassung vergrößert Windows ein {w} × {h}-Fenster auf {sw} × {sh}, und es ragt über den Bildschirm.",
    audioTitle: "Audio im Spiel",
    audioHint:
      "Für RS_ASIO empfohlen: exklusiver Modus und Ultra-Low-Latency an, Puffer so klein wie ohne Knacken möglich.",
    exclusiveMode: "Exklusiver Modus",
    ullMode: "Ultra-Low-Latency-Modus",
    latencyBuffer: "Latenzpuffer",
    bufferLowest: "1 (niedrigste Latenz)",
    bufferStablest: "4 (stabilste)",
    saveHint: "Vor dem Speichern wird die bisherige Rocksmith.ini gesichert.",
  },
  audio: {
    applying: "Audio wird eingerichtet …",
    applied: "Audio-Einstellungen gespeichert.",
    savingProfile: "Profil wird gespeichert …",
    profileSaved: "Profil „{name}“ gespeichert.",
    notInstalledTitle: "RS_ASIO ist noch nicht installiert",
    notInstalledText: "Damit das Spiel dein Audio-Interface nutzen kann, installiere zuerst RS_ASIO.",
    gotoComponents: "Zu den Komponenten",
    searchingDrivers: "Suche Audio-Treiber …",
    noDriverTitle: "Kein ASIO-Treiber gefunden",
    linuxHint: "Unter Linux läuft ASIO über wineasio im Proton-Prefix. Diese Einrichtung wird später unterstützt.",
    noDriverText:
      "Installiere den Treiber deines Audio-Interfaces (z. B. „Focusrite Control“ für Scarlett-Geräte), schließ das Gerät an und öffne diesen Tab erneut. Ohne Interface-Treiber hilft ASIO4ALL.",
    device: "Gerät",
    profile: "Geräteprofil",
    customSettings: "Eigene Einstellungen",
    matchingProfiles: "Passend zu deinen Treibern",
    allProfiles: "Alle Profile",
    ownSuffix: " (eigenes)",
    driver: "ASIO-Treiber",
    driverMissing: "{name} (nicht installiert)",
    only64Suffix: " (nur 64-Bit)",
    only64Hint:
      "Dieser Treiber ist nur als 64-Bit-Version installiert. Rocksmith ist ein 32-Bit-Spiel und braucht den 32-Bit-Treiber.",
    inputChannel: "Eingangskanal Gitarre/Bass",
    outputChannel: "Erster Ausgangskanal",
    bufferSize: "Puffergröße",
    bufferDriver: "vom Treiber",
    bufferHost: "vom Spiel",
    bufferCustom: "fest",
    bufferHint: "Samples; bei Knacken erhöhen (96, 192 …)",
    output: "Tonausgabe",
    outAsio: "Über das Interface",
    outAsioText:
      "Kopfhörer/Boxen am Interface. Niedrigste Latenz – empfohlen zum Spielen. Discord kann den Spielton dann nicht übertragen; Sprache aus Discord hörst du, wenn Discord ebenfalls das Interface als Ausgabe nutzt.",
    outWindows: "Über Windows (z. B. Bluetooth)",
    outWindowsText:
      "Spielton über das Windows-Standardgerät, Gitarre weiter über das Interface. Läuft parallel zu Discord und lässt sich streamen. Bluetooth verzögert den Ton um etwa 0,2–0,5 s – auch deine Gitarre hörst du so spät.",
    outAsk: "Bei jedem Spielstart fragen",
    outAskText: "RS_ASIO zeigt beim Start eine Auswahl zwischen Interface und Windows.",
    windowsOutput: "Windows-Ausgabe:",
    recheck: "neu prüfen",
    bluetoothHint:
      "Das ist ein Bluetooth-Gerät. Spielton und deine Gitarre kommen damit 0,2–0,5 s verzögert an – zum Spielen merklich. Besser: Kopfhörer per Kabel an das Interface, oder Lautsprecher/Monitor als Windows-Ausgabe.",
    exclusiveHint:
      "Rocksmith beansprucht das Windows-Ausgabegerät gerade exklusiv. Dann hörst du in Discord niemanden, und Discord kann den Spielton nicht übertragen. Klick auf „Übernehmen“, um das zu beheben.",
    apply: "Übernehmen",
    applyHint:
      "Schreibt RS_ASIO.ini und passt Rocksmith.ini an (exklusiver Modus nur bei Ausgabe über das Interface). Beides wird vorher gesichert.",
    saveProfileTitle: "Als eigenes Profil speichern",
    saveProfileText: "Für Geräte ohne fertiges Profil: Einstellungen oben testen und hier für später speichern.",
    profileNamePlaceholder: "Name, z. B. Mein Interface",
  },
  songs: {
    installed: "Installiert: {name}",
    skipped: "Übersprungen: {name} – {reason}",
    nothingFound: "{source}: nichts gefunden",
    installing: "Songs werden installiert …",
    pickTitle: "Songs auswählen",
    filterName: "Rocksmith-Songs",
    enabling: "Song wird aktiviert …",
    disabling: "Song wird deaktiviert …",
    deleteConfirm: "„{title}“ endgültig löschen?",
    deleteTitle: "Song löschen",
    deleting: "Song wird gelöscht …",
    enablerConfirm:
      "GuitarStar lädt den offiziellen CDLC-Enabler-Installer von CustomsForge herunter und startet ihn. Windows fragt nach Administratorrechten. Im Installer die Option zum Freischalten von CDLC wählen. Weiter?",
    enablerTitle: "CDLC-Enabler",
    enablerMissing: "CDLC-Enabler fehlt",
    enablerMissingText:
      "Ohne ihn lädt Rocksmith keine eigenen Songs. Er kommt von CustomsForge und wird mit deren Installer eingerichtet.",
    cherubHint:
      "Hinweis: Der kostenlose DLC „Cherub Rock“ ist nicht installiert. Ältere Enabler brauchen ihn; in Steam unter Rocksmith → DLC aktivieren.",
    installEnabler: "CDLC-Enabler installieren …",
    find: "Songs finden",
    openCustomsforge: "CustomsForge öffnen",
    pickFiles: "Dateien auswählen …",
    findText:
      "In CustomsForge mit deinem (kostenlosen) Konto einloggen und Songs herunterladen – sie werden automatisch installiert. Oder Dateien (.psarc, .zip) einfach auf dieses Fenster ziehen.",
    yours: "Deine Songs",
    enabledCount: "{count} aktiv",
    disabledCount: "{count} deaktiviert",
    search: "Suchen …",
    empty:
      "Noch keine eigenen Songs. GuitarStar verwaltet nur Songs im Ordner dlc\\GuitarStar – die offiziellen Songs aus Steam bleiben unberührt.",
    colActive: "Aktiv",
    colArtist: "Interpret",
    colTitle: "Titel",
    colVersion: "Version",
    colSize: "Größe",
    enable: "Aktivieren",
    disable: "Deaktivieren",
    delete: "löschen",
    changesHint: "Änderungen gelten ab dem nächsten Spielstart (oder nach „Songs neu laden“ mit RSMods).",
    loading: "Lade Songs …",
    drop: "Loslassen, um die Songs zu installieren",
  },
  browser: {
    downloading: "Lädt {name} …",
    noSong: "kein Song gefunden",
    back: "Zurück",
    forward: "Vor",
    home: "Startseite",
    hint: "Heruntergeladene Songs werden automatisch installiert. Werbung auf dieser Seite schaltet CustomsForge (ausblendbar unter Einstellungen).",
    done: "Fertig",
  },
  components: {
    rsmodsConfirm:
      "GuitarStar lädt den offiziellen RSMods-Installer von GitHub (Lovrom8/RSMods) und startet ihn. Vorher werden xinput1_3.dll, RSMods.ini und cache.psarc gesichert. Weiter?",
    rsmodsConfirmTitle: "RSMods installieren",
    rsmodsLoading: "RSMods-Installer wird geladen …",
    installing: "{name} {version} wird installiert …",
    installedDone: "{name} {version} ist installiert.",
    removing: "{name} wird entfernt …",
    removed: "{name} wurde entfernt.",
    installedBadge: "installiert: {version}",
    updateBadge: "Update: {version}",
    source: "Quelle:",
    bundled: "In GuitarStar enthalten (Version {version})",
    conflict:
      "Im Spielordner liegt bereits eine andere {files} (vermutlich von einem anderen Mod). Beim Installieren wird sie gesichert und ersetzt; über den Verlauf lässt sie sich zurückholen.",
    filesMissing:
      "Dateien fehlen im Spielordner, vermutlich hat Steam die Spieldateien überprüft. Installiere die Version erneut.",
    updateTo: "Auf {version} aktualisieren",
    installVersion: "{version} installieren",
    searchVersions: "Nach Versionen suchen",
    reinstall: "Erneut installieren",
    colVersion: "Version",
    colPublished: "Veröffentlicht",
    prerelease: " (Vorabversion)",
    offline: "offline verfügbar",
    active: "aktiv",
    switchVersion: "Zu dieser Version wechseln",
    showLess: "weniger anzeigen",
    showAll: "alle {count} Versionen anzeigen",
    installedAt: "Installiert am {date}",
    cacheModifiedBadge: "Spieldatei-Mods aktiv (z. B. Fast Load)",
    optional: "optional",
    cacheModified:
      "cache.psarc ist gegenüber dem gesicherten Original verändert – Fast Load oder ein anderer „Set And Forget“-Mod ist angewendet.",
    cacheOriginal:
      "cache.psarc ist noch im Originalzustand: Fast Load ist nicht angewendet. In RSMods unter „Set And Forget Mods“ einschalten.",
    cacheUnknown:
      "Ob Fast Load aktiv ist, lässt sich nicht prüfen: Es gibt keine von GuitarStar gesicherte Original-cache.psarc.",
    rsmodsIntroBefore: "Große Mod-Sammlung der Community:",
    rsmodsIntroAfter:
      "(überspringt die Intro-Logos beim Start), eigene Songlisten, Songs ohne Neustart neu laden, Riff Repeater über 100 %, Lautstärke per Hotkey und mehr. Eingestellt wird alles in der RSMods-Oberfläche.",
    rsmodsSourceNote: "– wird mit dem offiziellen Installer eingerichtet, nicht von GuitarStar mitgeliefert.",
    rsmodsHint:
      "Fast Load und einige andere Funktionen verändern die Spieldatei cache.psarc. GuitarStar sichert sie vor der Installation; über „Verlauf“ lässt sich der Stand davor zurückholen. Fast Load gilt auf HDDs als instabil.",
    installLatest: "Neueste Version installieren …",
    installRsmods: "RSMods installieren …",
    opening: "RSMods wird geöffnet …",
    openRsmods: "RSMods öffnen (Fast Load einschalten)",
  },
  history: {
    undoOne: "„{label}“ rückgängig machen?",
    undoMany: "„{label}“ und die {count} späteren Änderungen rückgängig machen?",
    undoTitle: "Rückgängig machen",
    undoing: "Wird rückgängig gemacht …",
    undone: "Rückgängig gemacht.",
    restoreConfirm:
      "Alle Änderungen von GuitarStar am Spielordner zurücknehmen? RS_ASIO und alle Einstellungen werden auf den Stand vor der ersten Änderung gesetzt.",
    originalTitle: "Originalzustand",
    restoring: "Originalzustand wird wiederhergestellt …",
    restored: "Originalzustand wiederhergestellt.",
    title: "Verlauf der Änderungen",
    intro:
      "Vor jeder Änderung am Spielordner sichert GuitarStar die betroffenen Dateien. Du kannst jede Änderung samt allen späteren zurücknehmen.",
    empty: "Noch keine Änderungen.",
    files: " · {count} Datei(en)",
    windowsSetting: " · Windows-Einstellung",
    undo: "Rückgängig",
    originalText: "Nimmt alle Änderungen von GuitarStar zurück, z. B. wenn das Spiel nicht mehr startet.",
    restoreButton: "Originalzustand wiederherstellen",
  },
  settings: {
    language: "Sprache / Language",
    hideAdsOn: "Werbung wird beim nächsten Öffnen von CustomsForge ausgeblendet.",
    hideAdsOff: "Werbung wird wieder angezeigt.",
    adsNotOurs: "Die Werbung kommt von CustomsForge, nicht von GuitarStar.",
    adsText:
      "GuitarStar zeigt die Seite von CustomsForge nur an und verdient daran nichts. Die Werbung schaltet CustomsForge selbst – zusammen mit Spenden bezahlt sie den Betrieb der Seite, über die die kostenlosen Songs (CDLC) der Community verteilt werden.",
    adsSupportText:
      "Du kannst die Werbung hier ausblenden. Wenn du das tust, überleg dir, CustomsForge auf anderem Weg zu unterstützen.",
    support: "CustomsForge unterstützen",
    hideAds: "Werbung auf CustomsForge ausblenden",
    hideAdsHint:
      "Gilt ab dem nächsten Öffnen von CustomsForge. Du bleibst dabei eingeloggt. Falls nach einer Änderung der Seite doch Werbung durchrutscht oder etwas fehlt, gib bitte auf GitHub Bescheid.",
    troubleshooting: "Fehlersuche",
    nextLaunch: "Gilt ab dem nächsten Spielstart.",
    dllLog: "Diagnose-Protokolle des Skalierungs-Fix schreiben",
    dllLogHint:
      "Schreibt beim Spielen Fenster- und Video-Ereignisse in GuitarStar-window.log im Spielordner. Nur für die Fehlersuche nötig – sonst aus lassen.",
    dllLogUnavailable: "Erst verfügbar, wenn der Skalierungs-Fix installiert ist.",
  },
};

type Dict = typeof de;
type DeepPartial<T> = { [K in keyof T]?: T[K] extends string ? string : DeepPartial<T[K]> };
type Leaves<T> = {
  [K in keyof T & string]: T[K] extends string ? K : `${K}.${Leaves<T[K]>}`;
}[keyof T & string];

/** Every valid `t()` key, e.g. "game.resolution". */
export type MessageKey = Leaves<Dict>;

const en: DeepPartial<Dict> = {
  common: {
    save: "Save",
    saving: "Saving …",
    recheck: "Check again",
    remove: "Remove",
    install: "Install",
    installed: "installed",
    notInstalled: "not installed",
    close: "close",
    reload: "Reload",
    installerLoading: "Downloading installer …",
    installerStarted: "Installer started. Afterwards, click “Check again” here.",
  },
  app: {
    tabGame: "Game",
    tabAudio: "Audio",
    tabSongs: "Songs",
    tabComponents: "Components",
    tabHistory: "History",
    tabSettings: "Settings",
    chooseFolderTitle: "Select Rocksmith 2014 folder",
    checkingFolder: "Checking folder …",
    change: "change",
    loading: "Loading …",
    welcome: "Welcome to GuitarStar",
    intro: "First, select your Rocksmith 2014 folder. GuitarStar only changes files there that it has backed up first.",
    detected: "Installations found:",
    noneDetected: "No installation was found through Steam.",
    chooseFolder: "Select folder …",
  },
  update: {
    available: "GuitarStar {version} is available",
    current: "(installed: {version})",
    failed: "Update failed: {error}",
    downloading: "Downloading … {progress}%",
    now: "Update now",
    later: "later",
  },
  game: {
    keepVisibleOn: "From the next launch on, Rocksmith no longer minimizes.",
    keepVisibleOff: "Rocksmith behaves as before again.",
    dpiRemoving: "Removing setting …",
    dpiRemoved: "The ineffective DPI setting was removed.",
    savingSettings: "Saving settings …",
    settingsSaved: "Settings saved.",
    installingFix: "Installing scaling fix …",
    fixInstalledDone: "Scaling fix installed. Restart Rocksmith.",
    dpiTitle: "Remove DPI setting",
    dpiText:
      "The Windows setting “Scaling performed by: Application” is set for Rocksmith, but has no effect on the Steam version. It can be removed safely; the scaling fix below replaces it.",
    scalingTitle: "Windows scaling",
    fixActive: "Scaling fix active",
    fixActiveText:
      "Rocksmith gets your monitor's real resolution – windowed, in a fullscreen window and in exclusive fullscreen. You can remove the fix under “Components”.",
    updateFix: "Update fix to {version}",
    scaledText:
      "Your monitor is scaled. Rocksmith can't handle that: Windows enlarges the game and it extends beyond the screen. The scaling fix solves this in all display modes.",
    fixConflict: "Note: the game folder already contains a different {files}. It will be backed up and replaced.",
    installFix: "Install scaling fix",
    iniMissingTitle: "Rocksmith.ini is missing",
    iniMissingText:
      "The game only creates its settings file on first launch. Start Rocksmith once via Steam, quit it again and then reload this page.",
    checking: "Checking …",
    display: "Display",
    resolution: "Resolution",
    monitorSuffix: " (monitor)",
    displayMode: "Display mode",
    windowed: "Windowed",
    fullscreenWindow: "Fullscreen window",
    exclusiveFullscreen: "Exclusive fullscreen",
    focusLoss: "On focus loss",
    keepVisible: "don't minimize (e.g. when clicking on the second monitor)",
    exclusiveHint:
      "In exclusive fullscreen, DirectX always minimizes the game when it loses focus – this can't be prevented. For two monitors, choose “Fullscreen window”; thanks to the scaling fix it looks the same.",
    adapt: "Without fix: adapt resolution to Windows scaling ({percent}%) (windowed mode only)",
    adaptOn:
      "Rocksmith gets {w} × {h}; Windows then enlarges the window to exactly {tw} × {th}. The image is slightly softer as a result.",
    adaptOff: "Without adapting, Windows enlarges a {w} × {h} window to {sw} × {sh}, and it extends beyond the screen.",
    audioTitle: "In-game audio",
    audioHint:
      "Recommended for RS_ASIO: exclusive mode and ultra low latency on, buffer as small as possible without crackling.",
    exclusiveMode: "Exclusive mode",
    ullMode: "Ultra low latency mode",
    latencyBuffer: "Latency buffer",
    bufferLowest: "1 (lowest latency)",
    bufferStablest: "4 (most stable)",
    saveHint: "The current Rocksmith.ini is backed up before saving.",
  },
  audio: {
    applying: "Setting up audio …",
    applied: "Audio settings saved.",
    savingProfile: "Saving profile …",
    profileSaved: "Profile “{name}” saved.",
    notInstalledTitle: "RS_ASIO is not installed yet",
    notInstalledText: "Install RS_ASIO first so the game can use your audio interface.",
    gotoComponents: "Go to Components",
    searchingDrivers: "Looking for audio drivers …",
    noDriverTitle: "No ASIO driver found",
    linuxHint: "On Linux, ASIO runs through wineasio in the Proton prefix. Setting this up will be supported later.",
    noDriverText:
      "Install the driver for your audio interface (e.g. “Focusrite Control” for Scarlett devices), connect the device and open this tab again. Without an interface driver, ASIO4ALL can help.",
    device: "Device",
    profile: "Device profile",
    customSettings: "Custom settings",
    matchingProfiles: "Matching your drivers",
    allProfiles: "All profiles",
    ownSuffix: " (custom)",
    driver: "ASIO driver",
    driverMissing: "{name} (not installed)",
    only64Suffix: " (64-bit only)",
    only64Hint:
      "This driver is only installed as a 64-bit version. Rocksmith is a 32-bit game and needs the 32-bit driver.",
    inputChannel: "Guitar/bass input channel",
    outputChannel: "First output channel",
    bufferSize: "Buffer size",
    bufferDriver: "from driver",
    bufferHost: "from game",
    bufferCustom: "fixed",
    bufferHint: "samples; increase if it crackles (96, 192 …)",
    output: "Sound output",
    outAsio: "Through the audio interface",
    outAsioText:
      "Headphones/speakers on the interface. Lowest latency – recommended for playing. Discord can't stream the game sound then; you hear voice from Discord if Discord also uses the interface as its output.",
    outWindows: "Through Windows (e.g. Bluetooth)",
    outWindowsText:
      "Game sound through the Windows default device, guitar still through the interface. Works alongside Discord and can be streamed. Bluetooth delays the sound by about 0.2–0.5 s – you hear your guitar that late, too.",
    outAsk: "Ask on every game launch",
    outAskText: "RS_ASIO shows a choice between interface and Windows at launch.",
    windowsOutput: "Windows output:",
    recheck: "check again",
    bluetoothHint:
      "This is a Bluetooth device. Game sound and your guitar arrive 0.2–0.5 s late – noticeable while playing. Better: wired headphones on the interface, or speakers/monitor as the Windows output.",
    exclusiveHint:
      "Rocksmith is currently claiming the Windows output device exclusively. You won't hear anyone in Discord, and Discord can't stream the game sound. Click “Apply” to fix this.",
    apply: "Apply",
    applyHint:
      "Writes RS_ASIO.ini and adjusts Rocksmith.ini (exclusive mode only with output through the interface). Both are backed up first.",
    saveProfileTitle: "Save as custom profile",
    saveProfileText: "For devices without a ready-made profile: test the settings above and save them here for later.",
    profileNamePlaceholder: "Name, e.g. My interface",
  },
  songs: {
    installed: "Installed: {name}",
    skipped: "Skipped: {name} – {reason}",
    nothingFound: "{source}: nothing found",
    installing: "Installing songs …",
    pickTitle: "Select songs",
    filterName: "Rocksmith songs",
    enabling: "Enabling song …",
    disabling: "Disabling song …",
    deleteConfirm: "Delete “{title}” permanently?",
    deleteTitle: "Delete song",
    deleting: "Deleting song …",
    enablerConfirm:
      "GuitarStar downloads the official CDLC Enabler installer from CustomsForge and starts it. Windows will ask for administrator rights. In the installer, choose the option to unlock CDLC. Continue?",
    enablerTitle: "CDLC Enabler",
    enablerMissing: "CDLC Enabler missing",
    enablerMissingText:
      "Without it, Rocksmith doesn't load custom songs. It comes from CustomsForge and is set up with their installer.",
    cherubHint:
      "Note: the free DLC “Cherub Rock” is not installed. Older enablers need it; enable it in Steam under Rocksmith → DLC.",
    installEnabler: "Install CDLC Enabler …",
    find: "Find songs",
    openCustomsforge: "Open CustomsForge",
    pickFiles: "Select files …",
    findText:
      "Log in to CustomsForge with your (free) account and download songs – they are installed automatically. Or just drag files (.psarc, .zip) onto this window.",
    yours: "Your songs",
    enabledCount: "{count} active",
    disabledCount: "{count} disabled",
    search: "Search …",
    empty:
      "No custom songs yet. GuitarStar only manages songs in the dlc\\GuitarStar folder – the official songs from Steam stay untouched.",
    colActive: "Active",
    colArtist: "Artist",
    colTitle: "Title",
    colVersion: "Version",
    colSize: "Size",
    enable: "Enable",
    disable: "Disable",
    delete: "delete",
    changesHint: "Changes take effect on the next game launch (or after “Reload songs” with RSMods).",
    loading: "Loading songs …",
    drop: "Drop to install the songs",
  },
  browser: {
    downloading: "Downloading {name} …",
    noSong: "no song found",
    back: "Back",
    forward: "Forward",
    home: "Home page",
    hint: "Downloaded songs are installed automatically. Ads on this site are run by CustomsForge (can be hidden in Settings).",
    done: "Done",
  },
  components: {
    rsmodsConfirm:
      "GuitarStar downloads the official RSMods installer from GitHub (Lovrom8/RSMods) and starts it. xinput1_3.dll, RSMods.ini and cache.psarc are backed up first. Continue?",
    rsmodsConfirmTitle: "Install RSMods",
    rsmodsLoading: "Downloading RSMods installer …",
    installing: "Installing {name} {version} …",
    installedDone: "{name} {version} is installed.",
    removing: "Removing {name} …",
    removed: "{name} was removed.",
    installedBadge: "installed: {version}",
    updateBadge: "Update: {version}",
    source: "Source:",
    bundled: "Included in GuitarStar (version {version})",
    conflict:
      "The game folder already contains a different {files} (probably from another mod). Installing backs it up and replaces it; you can get it back via History.",
    filesMissing:
      "Files are missing from the game folder, probably because Steam verified the game files. Install the version again.",
    updateTo: "Update to {version}",
    installVersion: "Install {version}",
    searchVersions: "Check for versions",
    reinstall: "Reinstall",
    colVersion: "Version",
    colPublished: "Published",
    prerelease: " (pre-release)",
    offline: "available offline",
    active: "active",
    switchVersion: "Switch to this version",
    showLess: "show less",
    showAll: "show all {count} versions",
    installedAt: "Installed on {date}",
    cacheModifiedBadge: "Game file mods active (e.g. Fast Load)",
    optional: "optional",
    cacheModified:
      "cache.psarc differs from the backed-up original – Fast Load or another “Set And Forget” mod is applied.",
    cacheOriginal:
      "cache.psarc is still in its original state: Fast Load is not applied. Turn it on in RSMods under “Set And Forget Mods”.",
    cacheUnknown:
      "Can't check whether Fast Load is active: there is no original cache.psarc backed up by GuitarStar.",
    rsmodsIntroBefore: "Large community mod collection:",
    rsmodsIntroAfter:
      "(skips the intro logos at launch), custom song lists, reloading songs without a restart, Riff Repeater above 100%, volume via hotkey and more. Everything is configured in the RSMods interface.",
    rsmodsSourceNote: "– set up with the official installer, not shipped with GuitarStar.",
    rsmodsHint:
      "Fast Load and some other features modify the game file cache.psarc. GuitarStar backs it up before installing; you can restore the previous state via “History”. Fast Load is considered unstable on HDDs.",
    installLatest: "Install latest version …",
    installRsmods: "Install RSMods …",
    opening: "Opening RSMods …",
    openRsmods: "Open RSMods (turn on Fast Load)",
  },
  history: {
    undoOne: "Roll back “{label}”?",
    undoMany: "Roll back “{label}” and the {count} later change(s)?",
    undoTitle: "Roll back",
    undoing: "Rolling back …",
    undone: "Rolled back.",
    restoreConfirm:
      "Revert all changes GuitarStar made to the game folder? RS_ASIO and all settings are reset to the state before the first change.",
    originalTitle: "Original state",
    restoring: "Restoring original state …",
    restored: "Original state restored.",
    title: "History of changes",
    intro:
      "Before every change to the game folder, GuitarStar backs up the affected files. You can roll back any change together with all later ones.",
    empty: "No changes yet.",
    files: " · {count} file(s)",
    windowsSetting: " · Windows setting",
    undo: "Roll back",
    originalText: "Reverts all changes made by GuitarStar, e.g. if the game no longer starts.",
    restoreButton: "Restore original state",
  },
  settings: {
    language: "Sprache / Language",
    hideAdsOn: "Ads will be hidden the next time CustomsForge is opened.",
    hideAdsOff: "Ads are shown again.",
    adsNotOurs: "The ads come from CustomsForge, not from GuitarStar.",
    adsText:
      "GuitarStar only displays the CustomsForge site and earns nothing from it. CustomsForge runs the ads itself – together with donations, they pay for running the site through which the community's free songs (CDLC) are shared.",
    adsSupportText: "You can hide the ads here. If you do, please consider supporting CustomsForge in another way.",
    support: "Support CustomsForge",
    hideAds: "Hide ads on CustomsForge",
    hideAdsHint:
      "Takes effect the next time CustomsForge is opened. You stay logged in. If ads slip through or something is missing after the site changes, please let us know on GitHub.",
    troubleshooting: "Troubleshooting",
    nextLaunch: "Takes effect on the next game launch.",
    dllLog: "Write diagnostic logs of the scaling fix",
    dllLogHint:
      "While playing, writes window and video events to GuitarStar-window.log in the game folder. Only needed for troubleshooting – otherwise leave it off.",
    dllLogUnavailable: "Only available once the scaling fix is installed.",
  },
};

const dictionaries: Record<Lang, DeepPartial<Dict>> = { de, en };
const LOCALES: Record<Lang, string> = { de: "de-DE", en: "en-US" };

function initialLang(): Lang {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "de" || saved === "en") return saved;
  } catch {
    // Storage unavailable: fall through to the browser language.
  }
  return typeof navigator !== "undefined" && navigator.language?.toLowerCase().startsWith("de") ? "de" : "en";
}

const current = $state<{ lang: Lang }>({ lang: initialLang() });

function applyHtmlLang() {
  if (typeof document !== "undefined") document.documentElement.lang = current.lang;
}
applyHtmlLang();

export function getLang(): Lang {
  return current.lang;
}

export function setLang(lang: Lang) {
  current.lang = lang;
  applyHtmlLang();
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
  const text = lookup(dictionaries[current.lang], key) ?? lookup(de, key) ?? key;
  if (!params) return text;
  return text.replace(/\{(\w+)\}/g, (match, name: string) => (name in params ? String(params[name]) : match));
}

/** BCP 47 locale for dates and numbers in the current language. */
export function locale(): string {
  return LOCALES[current.lang];
}

export function formatNumber(value: number, options?: Intl.NumberFormatOptions): string {
  return value.toLocaleString(locale(), options);
}
