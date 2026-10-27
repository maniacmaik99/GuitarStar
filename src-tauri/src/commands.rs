//! Commands called from the UI. Every operation that changes the game folder
//! runs under `Ctx::op`, so two clicks can never modify files at the same time.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::State;

use guitarstar_core::audio::{self, AsioDriver, AudioConfig, DeviceProfile};
use guitarstar_core::components::{self, CachedSource, ComponentDef, ReleaseInfo};
use guitarstar_core::error::{AppError, Result};
use guitarstar_core::game::{self, GameInfo, GameSettings};
use guitarstar_core::i18n::{self, Label, Lang};
use guitarstar_core::snapshot::{SnapshotStore, SnapshotWriter};
use guitarstar_core::state::{AppState, InstalledComponent};
use guitarstar_core::sources::{self, PendingDownload, Sources};
use guitarstar_core::versions::{self, VersionStore};
use guitarstar_core::{cdlc, display, linux, reqwest, rsmods, saves, steam, tr};

const RS_ASIO_INI: &str = "RS_ASIO.ini";

pub struct Ctx {
    pub data_dir: PathBuf,
    pub state: Mutex<AppState>,
    pub op: tokio::sync::Mutex<()>,
    pub http: reqwest::Client,
}

impl Ctx {
    pub fn new(data_dir: PathBuf) -> Self {
        let state = AppState::load(&data_dir);
        if let Some(lang) = state.lang {
            i18n::set_lang(lang);
        }
        Ctx {
            data_dir,
            state: Mutex::new(state),
            op: tokio::sync::Mutex::new(()),
            http: components::http_client(),
        }
    }

    fn lock(&self) -> MutexGuard<'_, AppState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn state(&self) -> AppState {
        self.lock().clone()
    }

    fn save_state(&self, s: AppState) -> Result<()> {
        s.save(&self.data_dir)?;
        *self.lock() = s;
        Ok(())
    }

    fn game_dir(&self) -> Result<PathBuf> {
        self.state()
            .game_dir
            .map(PathBuf::from)
            .ok_or_else(|| AppError::msg(tr!("game.chooseFirst")))
    }

    fn snapshots(&self) -> SnapshotStore {
        SnapshotStore::new(self.data_dir.join("snapshots"))
    }

    fn profiles_dir(&self) -> PathBuf {
        self.data_dir.join("profiles")
    }

    fn save_backups_dir(&self) -> PathBuf {
        self.data_dir.join("save-backups")
    }

    /// Backs up every Rocksmith save before a risky action. Without a save
    /// folder there is nothing to protect; if one exists but cannot be copied,
    /// the action is aborted.
    fn backup_saves_before(&self, label: Label) -> Result<()> {
        for folder in saves::find_save_folders().into_iter().filter(|f| f.file_count > 0) {
            saves::backup(Path::new(&folder.path), &self.save_backups_dir(), &label, true)
                .map_err(|e| AppError::msg(tr!("saves.backupFailed", error = e)))?;
        }
        Ok(())
    }

    /// Runs `change` inside a snapshot; on error the partial change is rolled back.
    fn change_game_files(
        &self,
        label: Label,
        change: impl FnOnce(&Path, &mut SnapshotWriter) -> Result<()>,
    ) -> Result<()> {
        let game = self.game_dir()?;
        let mut w = self.snapshots().begin(&game, label, serde_json::to_value(self.state())?)?;
        match change(&game, &mut w) {
            Ok(()) => {
                w.commit()?;
                Ok(())
            }
            Err(e) => {
                w.abort()?;
                Err(e)
            }
        }
    }
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

#[derive(Serialize)]
pub struct SnapshotSummary {
    id: String,
    created: u64,
    /// Text in the language at creation time; the UI prefers `label_key`.
    label: String,
    label_key: Option<String>,
    label_args: Option<BTreeMap<String, String>>,
    file_count: usize,
}

#[derive(Serialize)]
pub struct ComponentStatus {
    id: &'static str,
    /// Name and description in the current language.
    name: String,
    description: String,
    repo: Option<&'static str>,
    /// Set for components shipped inside GuitarStar.
    bundled_version: Option<&'static str>,
    installed: Option<InstalledComponent>,
    /// False when an installed file has vanished, e.g. after Steam verified the game files.
    files_present: bool,
    /// Files of the same name that another mod already put into the game folder.
    conflicts: Vec<String>,
    cached: Vec<CachedSource>,
}

#[derive(Serialize)]
pub struct Overview {
    platform: &'static str,
    state: AppState,
    game: Option<GameInfo>,
    settings: Option<GameSettings>,
    /// Actual registry state; `None` where the setting does not exist (Linux).
    dpi_override: Option<bool>,
    components: Vec<ComponentStatus>,
    snapshots: Vec<SnapshotSummary>,
}

fn game_exe(game_dir: &Path) -> PathBuf {
    game_dir.join(steam::ROCKSMITH_EXE)
}

#[tauri::command]
pub fn overview(ctx: State<'_, Ctx>) -> Result<Overview> {
    let state = ctx.state();
    let game_dir = state.game_dir.as_ref().map(PathBuf::from);
    let game = game_dir.as_deref().map(game::inspect);
    let settings = match &game_dir {
        Some(d) => game::read_settings(d)?,
        None => None,
    };
    let components = components::COMPONENTS
        .iter()
        .map(|def| {
            let installed = state.installed.get(def.id).cloned();
            let files_present = match (&installed, &game_dir) {
                (Some(i), Some(d)) => i.files.iter().all(|f| d.join(f).is_file()),
                _ => true,
            };
            let conflicts = match (&installed, &game_dir) {
                (None, Some(d)) => def
                    .files
                    .iter()
                    .filter(|f| d.join(f).is_file() && !components::is_bundled_file(def, f, &d.join(f)))
                    .map(|f| f.to_string())
                    .collect(),
                _ => Vec::new(),
            };
            ComponentStatus {
                id: def.id,
                name: def.name(),
                description: def.description(),
                repo: def.repo(),
                bundled_version: match def.source {
                    components::Source::Bundled { version, .. } => Some(version),
                    components::Source::GitHub { .. } => None,
                },
                installed,
                files_present,
                conflicts,
                cached: components::cached_versions(&ctx.data_dir, def.id),
            }
        })
        .collect();
    let snapshots = ctx
        .snapshots()
        .list()?
        .into_iter()
        .map(|s| {
            // Entries from before labels had keys are plain German text.
            let (label_key, label_args) = match s.label_key {
                Some(key) => (Some(key), s.label_args),
                None => i18n::legacy_label(&s.label).map_or((None, None), |l| (l.key(), l.args())),
            };
            SnapshotSummary { file_count: s.files.len(), id: s.id, created: s.created, label: s.label, label_key, label_args }
        })
        .collect();
    Ok(Overview {
        platform: std::env::consts::OS,
        dpi_override: game_dir.as_deref().and_then(|d| display::dpi_override(&game_exe(d))),
        state,
        game,
        settings,
        components,
        snapshots,
    })
}

/// The UI's language: messages from here on are in it, and it is remembered
/// for the next start.
#[tauri::command]
pub fn set_language(ctx: State<'_, Ctx>, lang: String) -> Result<()> {
    let lang = Lang::parse(&lang).ok_or_else(|| AppError::msg(tr!("app.unknownLanguage", lang = lang)))?;
    i18n::set_lang(lang);
    let mut s = ctx.state();
    if s.lang != Some(lang) {
        s.lang = Some(lang);
        ctx.save_state(s)?;
    }
    Ok(())
}

#[tauri::command]
pub fn detect_games() -> Vec<String> {
    steam::find_rocksmith_installs()
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}

#[tauri::command]
pub fn set_game_dir(ctx: State<'_, Ctx>, path: String) -> Result<GameInfo> {
    let info = game::inspect(Path::new(&path));
    if !info.valid {
        return Err(AppError::msg(info.problems.join(" ")));
    }
    let mut s = ctx.state();
    s.game_dir = Some(path);
    ctx.save_state(s)?;
    Ok(info)
}

#[tauri::command]
pub async fn save_game_settings(ctx: State<'_, Ctx>, settings: GameSettings) -> Result<()> {
    let _op = ctx.op.lock().await;
    ctx.change_game_files(Label::new("labels.gameSettings"), |game, w| {
        game::write_settings(game, &settings, w)
    })
}

#[tauri::command]
pub fn list_profiles(ctx: State<'_, Ctx>) -> Result<Vec<DeviceProfile>> {
    audio::list_profiles(&ctx.profiles_dir())
}

#[tauri::command]
pub fn save_profile(ctx: State<'_, Ctx>, profile: DeviceProfile) -> Result<()> {
    audio::save_user_profile(&ctx.profiles_dir(), &profile)
}

#[tauri::command]
pub fn list_asio_drivers(ctx: State<'_, Ctx>) -> Vec<AsioDriver> {
    let game_dir = ctx.state().game_dir.map(PathBuf::from);
    audio::asio_drivers_for(game_dir.as_deref())
}

/// Proton setup (wineasio, Steam launch options); `None` on Windows or
/// before a game folder is chosen.
#[tauri::command]
pub fn linux_setup_status(ctx: State<'_, Ctx>) -> Option<linux::LinuxSetupStatus> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let game_dir = ctx.state().game_dir.map(PathBuf::from)?;
    Some(linux::setup_status(&game_dir))
}

#[tauri::command]
pub async fn default_output_device() -> Option<audio::OutputDevice> {
    audio::default_output()
}

/// RS_ASIO's own template from the installed version, or the bundled copy.
fn rs_asio_template(ctx: &Ctx) -> String {
    let state = ctx.state();
    state
        .installed
        .get("rs_asio")
        .and_then(|i| fs::read_to_string(components::cache_dir(&ctx.data_dir, "rs_asio", &i.version).join(RS_ASIO_INI)).ok())
        .unwrap_or_else(|| audio::RS_ASIO_INI_TEMPLATE.to_string())
}

fn write_rs_asio_ini(game: &Path, template: &str, cfg: &AudioConfig, w: &mut SnapshotWriter) -> Result<()> {
    w.protect(RS_ASIO_INI)?;
    fs::write(game.join(RS_ASIO_INI), audio::render_rs_asio_ini(template, cfg))?;
    Ok(())
}

#[tauri::command]
pub async fn apply_audio_config(ctx: State<'_, Ctx>, config: AudioConfig) -> Result<()> {
    let _op = ctx.op.lock().await;
    if !ctx.state().installed.contains_key("rs_asio") {
        return Err(AppError::msg(tr!("audio.installRsAsioFirst")));
    }
    let template = rs_asio_template(&ctx);
    ctx.change_game_files(Label::new("labels.audioSettings"), |game, w| {
        write_rs_asio_ini(game, &template, &config, w)?;
        // RS_ASIO's recommended game settings; only possible once the game created its ini.
        // Exclusive mode would lock a Windows output device for the game alone, so
        // Discord and the like could neither play on it nor capture the game sound.
        if game.join(game::ROCKSMITH_INI).is_file() {
            let s = GameSettings {
                exclusive_mode: Some(config.output_mode == audio::OutputMode::Asio),
                ultra_low_latency: Some(true),
                ..Default::default()
            };
            game::write_settings(game, &s, w)?;
        }
        Ok(())
    })?;
    let mut s = ctx.state();
    s.audio = Some(config);
    ctx.save_state(s)
}

#[tauri::command]
pub async fn component_releases(ctx: State<'_, Ctx>, id: String) -> Result<Vec<ReleaseInfo>> {
    components::releases(&ctx.http, components::find(&id)?).await
}

fn copy_component_files(def: &ComponentDef, from: &Path, game: &Path, w: &mut SnapshotWriter) -> Result<()> {
    for f in def.files {
        w.protect(f)?;
        fs::copy(from.join(f), game.join(f))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn install_component(ctx: State<'_, Ctx>, id: String, version: String) -> Result<()> {
    let _op = ctx.op.lock().await;
    let def = components::find(&id)?;
    ctx.game_dir()?;
    // Bundled files are rewritten every time, so a newer app build always installs its own copy.
    components::materialize_bundled(&ctx.data_dir, def)?;
    if !components::is_cached(&ctx.data_dir, def.id, &version) {
        let rel = components::releases(&ctx.http, def)
            .await?
            .into_iter()
            .find(|r| r.version == version)
            .ok_or_else(|| AppError::msg(tr!("components.notOnGithub", name = def.name(), version = version)))?;
        components::download(&ctx.http, &ctx.data_dir, def, &rel).await?;
    }
    let source = components::cache_dir(&ctx.data_dir, def.id, &version);
    let state = ctx.state();
    let label = match state.installed.get(def.id) {
        Some(prev) if prev.version != version => Label::new("labels.componentSwitched")
            .arg("component", def.id)
            .arg("from", &prev.version)
            .arg("to", &version),
        _ => Label::new("labels.componentInstalled").arg("component", def.id).arg("version", &version),
    };
    let mut files: Vec<String> = def.files.iter().map(|f| f.to_string()).collect();
    ctx.change_game_files(label, |game, w| {
        copy_component_files(def, &source, game, w)?;
        if def.id == "rs_asio" && !game.join(RS_ASIO_INI).is_file() {
            let template = fs::read_to_string(source.join(RS_ASIO_INI))
                .unwrap_or_else(|_| audio::RS_ASIO_INI_TEMPLATE.to_string());
            match &state.audio {
                Some(cfg) => write_rs_asio_ini(game, &template, cfg, w)?,
                None => {
                    w.protect(RS_ASIO_INI)?;
                    fs::write(game.join(RS_ASIO_INI), &template)?;
                }
            }
            files.push(RS_ASIO_INI.to_string());
        }
        Ok(())
    })?;
    let mut s = ctx.state();
    // Keep track of the ini from an earlier install of the same component.
    if let Some(prev) = s.installed.get(def.id) {
        for f in &prev.files {
            if !files.contains(f) {
                files.push(f.clone());
            }
        }
    }
    s.installed.insert(def.id.to_string(), InstalledComponent { version, files, installed_at: now() });
    ctx.save_state(s)
}

#[tauri::command]
pub async fn uninstall_component(ctx: State<'_, Ctx>, id: String) -> Result<()> {
    let _op = ctx.op.lock().await;
    let def = components::find(&id)?;
    let Some(installed) = ctx.state().installed.get(def.id).cloned() else {
        return Ok(());
    };
    ctx.change_game_files(Label::new("labels.componentRemoved").arg("component", def.id), |game, w| {
        for f in &installed.files {
            w.protect(f)?;
            let p = game.join(f);
            if p.is_file() {
                fs::remove_file(p)?;
            }
        }
        Ok(())
    })?;
    let mut s = ctx.state();
    s.installed.remove(def.id);
    ctx.save_state(s)
}

fn apply_restored_state(ctx: &Ctx, restored: serde_json::Value) -> Result<()> {
    let mut s: AppState = serde_json::from_value(restored).unwrap_or_default();
    // The selected folder and the language are not part of what gets undone.
    s.game_dir = ctx.state().game_dir;
    s.lang = ctx.state().lang;
    if s.dpi_override != ctx.state().dpi_override {
        display::set_dpi_override(&game_exe(&ctx.game_dir()?), s.dpi_override)?;
    }
    ctx.save_state(s)
}

/// Writes the DLL options from `s` and records the ini as part of the fix,
/// so removing the fix removes it too.
fn apply_dll_config(ctx: &Ctx, mut s: AppState, label: Label) -> Result<()> {
    let Some(fix) = s.installed.get_mut(components::DPI_FIX_ID) else {
        return Err(AppError::msg(tr!("display.installFixFirst")));
    };
    let cfg = game::DllConfig { keep_visible: s.keep_visible, log: s.dll_log };
    ctx.change_game_files(label, |game, w| game::write_dll_config(game, cfg, w))?;
    if !fix.files.iter().any(|f| f == game::GUITARSTAR_INI) {
        fix.files.push(game::GUITARSTAR_INI.to_string());
    }
    ctx.save_state(s)
}

#[tauri::command]
pub async fn set_keep_visible(ctx: State<'_, Ctx>, enabled: bool) -> Result<()> {
    let _op = ctx.op.lock().await;
    let mut s = ctx.state();
    s.keep_visible = enabled;
    let label = Label::new(if enabled { "labels.minimizeOff" } else { "labels.minimizeOn" });
    apply_dll_config(&ctx, s, label)
}

#[tauri::command]
pub async fn set_dll_log(ctx: State<'_, Ctx>, enabled: bool) -> Result<()> {
    let _op = ctx.op.lock().await;
    let mut s = ctx.state();
    s.dll_log = enabled;
    let label = Label::new(if enabled { "labels.dllLogOn" } else { "labels.dllLogOff" });
    apply_dll_config(&ctx, s, label)
}

#[tauri::command]
pub async fn set_dpi_override(ctx: State<'_, Ctx>, enabled: bool) -> Result<()> {
    let _op = ctx.op.lock().await;
    let game = ctx.game_dir()?;
    // Recorded as a snapshot without files so it shows up in the history and can be undone.
    let label = Label::new(if enabled { "labels.windowsScalingOff" } else { "labels.windowsScalingOn" });
    let w = ctx.snapshots().begin(&game, label, serde_json::to_value(ctx.state())?)?;
    display::set_dpi_override(&game_exe(&game), enabled)?;
    w.commit_always()?;
    let mut s = ctx.state();
    s.dpi_override = enabled;
    ctx.save_state(s)
}

#[tauri::command]
pub async fn restore_snapshot(ctx: State<'_, Ctx>, id: String) -> Result<()> {
    let _op = ctx.op.lock().await;
    let restored = ctx.snapshots().restore_to(&ctx.game_dir()?, &id)?;
    apply_restored_state(&ctx, restored)
}

#[tauri::command]
pub async fn restore_original(ctx: State<'_, Ctx>) -> Result<()> {
    let _op = ctx.op.lock().await;
    ctx.backup_saves_before(Label::new("labels.autoBeforeReset"))?;
    match ctx.snapshots().restore_original(&ctx.game_dir()?)? {
        Some(restored) => apply_restored_state(&ctx, restored),
        None => Ok(()),
    }
}

#[tauri::command]
pub fn save_folders() -> Vec<saves::SaveFolder> {
    saves::find_save_folders()
}

#[tauri::command]
pub fn save_backups(ctx: State<'_, Ctx>) -> Result<Vec<saves::BackupInfo>> {
    saves::list_backups(&ctx.save_backups_dir())
}

fn no_save_found() -> AppError {
    AppError::msg(tr!("saves.noneFound"))
}

#[tauri::command]
pub async fn backup_saves(ctx: State<'_, Ctx>, label: Option<String>) -> Result<Vec<saves::BackupInfo>> {
    let _op = ctx.op.lock().await;
    let folders = saves::find_save_folders();
    if folders.is_empty() {
        return Err(no_save_found());
    }
    // A label from the UI is already in its language.
    let label = label.map_or_else(|| Label::new("labels.manual"), Label::Plain);
    folders
        .iter()
        .map(|f| saves::backup(Path::new(&f.path), &ctx.save_backups_dir(), &label, false))
        .collect()
}

/// The daily backup on app start; returns whether one was made.
#[tauri::command]
pub async fn auto_backup_saves(ctx: State<'_, Ctx>) -> Result<bool> {
    let _op = ctx.op.lock().await;
    let root = ctx.save_backups_dir();
    let backups = saves::list_backups(&root)?;
    let mut made = false;
    for folder in saves::find_save_folders() {
        if saves::daily_backup_due(&folder, &backups, now()) {
            saves::backup(Path::new(&folder.path), &root, &Label::new("labels.autoOnStart"), true)?;
            made = true;
        }
    }
    Ok(made)
}

/// The save folder a backup belongs to: the current one of the same Steam
/// account, or the original location if Steam still has that account.
fn restore_target(backup: &saves::BackupInfo) -> Result<PathBuf> {
    if let Some(f) = saves::find_save_folders().into_iter().find(|f| f.account == backup.account) {
        return Ok(PathBuf::from(f.path));
    }
    let source = PathBuf::from(&backup.source);
    let account_dir = source.parent().and_then(Path::parent);
    match account_dir {
        Some(dir) if dir.is_dir() && saves::account_of(&source) == backup.account => Ok(source),
        _ => Err(no_save_found()),
    }
}

#[tauri::command]
pub async fn restore_save_backup(ctx: State<'_, Ctx>, id: String) -> Result<()> {
    let _op = ctx.op.lock().await;
    let root = ctx.save_backups_dir();
    let backup = saves::find_backup(&root, &id)?;
    saves::restore(&root, &id, &restore_target(&backup)?)?;
    Ok(())
}

#[tauri::command]
pub async fn delete_save_backup(ctx: State<'_, Ctx>, id: String) -> Result<()> {
    let _op = ctx.op.lock().await;
    saves::delete_backup(&ctx.save_backups_dir(), &id)
}

#[tauri::command]
pub fn open_save_folder(app: tauri::AppHandle, account: Option<String>) -> Result<()> {
    use tauri_plugin_opener::OpenerExt;
    let folder = saves::find_save_folders()
        .into_iter()
        .find(|f| account.as_ref().is_none_or(|a| &f.account == a))
        .ok_or_else(no_save_found)?;
    app.opener()
        .open_path(folder.path, None::<&str>)
        .map_err(|e| AppError::msg(tr!("saves.openFolderFailed", error = e)))
}

#[derive(Serialize)]
pub struct CdlcOverview {
    songs: Vec<cdlc::CdlcEntry>,
    enabler_installed: bool,
    /// `None` when the Steam manifest could not be read.
    cherub_rock_installed: Option<bool>,
}

#[tauri::command]
pub fn cdlc_overview(ctx: State<'_, Ctx>) -> Result<CdlcOverview> {
    let game = ctx.game_dir()?;
    Ok(CdlcOverview {
        songs: cdlc::list(&game, &ctx.data_dir)?,
        enabler_installed: cdlc::enabler_installed(&game),
        cherub_rock_installed: cdlc::cherub_rock_installed(&game),
    })
}

#[tauri::command]
pub async fn cdlc_import(ctx: State<'_, Ctx>, paths: Vec<String>) -> Result<cdlc::ImportResult> {
    let _op = ctx.op.lock().await;
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    cdlc::import(&ctx.game_dir()?, &ctx.data_dir, &paths)
}

#[tauri::command]
pub async fn cdlc_set_enabled(ctx: State<'_, Ctx>, name: String, enabled: bool) -> Result<()> {
    let _op = ctx.op.lock().await;
    cdlc::set_enabled(&ctx.game_dir()?, &ctx.data_dir, &name, enabled)
}

#[tauri::command]
pub async fn cdlc_delete(ctx: State<'_, Ctx>, cf: State<'_, CfDownloads>, name: String) -> Result<()> {
    let _op = ctx.op.lock().await;
    cdlc::delete(&ctx.game_dir()?, &ctx.data_dir, &name)?;
    cf.forget(&ctx.data_dir, &[name]);
    Ok(())
}

/// Enables, disables or deletes several songs at once.
#[tauri::command]
pub async fn cdlc_bulk(
    ctx: State<'_, Ctx>,
    cf: State<'_, CfDownloads>,
    names: Vec<String>,
    action: cdlc::BulkAction,
) -> Result<cdlc::BulkResult> {
    let _op = ctx.op.lock().await;
    let result = cdlc::bulk(&ctx.game_dir()?, &ctx.data_dir, &names, action);
    if action == cdlc::BulkAction::Delete {
        cf.forget(&ctx.data_dir, &result.done);
    }
    Ok(result)
}

/// Which CustomsForge song each downloaded file came from, by file name.
#[tauri::command]
pub fn cdlc_sources(ctx: State<'_, Ctx>, cf: State<'_, CfDownloads>) -> Sources {
    let _guard = cf.lock_sources();
    Sources::load(&ctx.data_dir)
}

/// The CustomsForge song of the most recent download click, and a lock for
/// `cdlc-sources.json` (downloads finish outside of `Ctx::op`).
#[derive(Default)]
pub struct CfDownloads {
    pending: Mutex<Option<PendingDownload>>,
    sources: Mutex<()>,
}

impl CfDownloads {
    fn lock_sources(&self) -> MutexGuard<'_, ()> {
        self.sources.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn note_request(&self, url: &reqwest::Url) {
        if let Some(cf_id) = sources::download_cf_id(url) {
            eprintln!("[customsforge] pending download of song {cf_id}");
            *self.pending.lock().unwrap_or_else(|e| e.into_inner()) =
                Some(PendingDownload { cf_id, requested_at: now() });
        }
    }

    /// Attributes freshly installed files to the pending CustomsForge song.
    fn record(&self, data_dir: &Path, imported: &[String], host: Option<&str>) {
        let _guard = self.lock_sources();
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        match sources::record_download(data_dir, imported, pending.as_ref(), now(), host) {
            Ok(true) => *pending = None,
            Ok(false) => {}
            Err(e) => eprintln!("[customsforge] could not store song source: {e}"),
        }
    }

    fn forget(&self, data_dir: &Path, names: &[String]) {
        let _guard = self.lock_sources();
        if let Err(e) = sources::forget(data_dir, names) {
            eprintln!("[customsforge] could not update song sources: {e}");
        }
    }
}

const ENABLER_URL: &str = "https://ignition4.customsforge.com/tools/download/cdlc-enabler";

/// Downloads CustomsForge's CDLC Enabler installer and starts it. The files it
/// is known to change are saved first, so the history can undo it.
#[tauri::command]
pub async fn install_cdlc_enabler(app: tauri::AppHandle, ctx: State<'_, Ctx>) -> Result<()> {
    use tauri_plugin_opener::OpenerExt;
    let _op = ctx.op.lock().await;
    ctx.backup_saves_before(Label::new("labels.autoBeforeEnabler"))?;
    let bytes = ctx.http.get(ENABLER_URL).send().await?.error_for_status()?.bytes().await?;
    if bytes.len() < 2 || &bytes[..2] != b"MZ" {
        return Err(AppError::msg(tr!("cdlc.enablerNoInstaller")));
    }
    let dir = ctx.data_dir.join("downloads");
    fs::create_dir_all(&dir)?;
    let installer = dir.join("RS2014-CDLC-Installer.exe");
    fs::write(&installer, &bytes)?;
    ctx.change_game_files(Label::new("labels.enablerBefore"), |_, w| {
        w.protect("D3DX9_42.dll")?;
        w.protect(steam::ROCKSMITH_EXE)
    })?;
    app.opener()
        .open_path(installer.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::msg(tr!("installer.startFailed", error = e)))
}

#[tauri::command]
pub async fn rsmods_status(ctx: State<'_, Ctx>) -> Result<rsmods::RsModsStatus> {
    let original = ctx.snapshots().original_copy("cache.psarc")?;
    Ok(rsmods::status(&ctx.game_dir()?, original.as_deref()))
}

/// Downloads the newest RSMods installer from GitHub and starts it, after
/// saving the files RSMods changes.
#[tauri::command]
pub async fn install_rsmods(app: tauri::AppHandle, ctx: State<'_, Ctx>) -> Result<String> {
    use tauri_plugin_opener::OpenerExt;
    let _op = ctx.op.lock().await;
    ctx.backup_saves_before(Label::new("labels.autoBeforeRsmods"))?;
    let rel = components::github_releases(&ctx.http, rsmods::REPO, rsmods::INSTALLER_SUFFIX)
        .await?
        .into_iter()
        .find(|r| !r.prerelease)
        .ok_or_else(|| AppError::msg(tr!("rsmods.noInstaller")))?;
    let bytes = components::download_verified(&ctx.http, &rel).await?;
    let dir = ctx.data_dir.join("downloads");
    fs::create_dir_all(&dir)?;
    let installer = dir.join(&rel.asset_name);
    fs::write(&installer, bytes)?;
    ctx.change_game_files(Label::new("labels.rsmodsBefore").arg("version", &rel.version), |_, w| {
        rsmods::PROTECTED_FILES.iter().try_for_each(|f| w.protect(f))
    })?;
    app.opener()
        .open_path(installer.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::msg(tr!("installer.startFailed", error = e)))?;
    Ok(rel.version)
}

/// Removes everything RSMods added and puts back the original cache.psarc.
/// Everything is saved first, so the removal itself can be undone.
#[tauri::command]
pub async fn uninstall_rsmods(ctx: State<'_, Ctx>) -> Result<()> {
    let _op = ctx.op.lock().await;
    ctx.backup_saves_before(Label::new("labels.autoBeforeRsmodsRemoval"))?;
    let original_cache = ctx.snapshots().original_copy("cache.psarc")?;
    ctx.change_game_files(Label::new("labels.rsmodsRemoved"), |game, w| {
        for f in rsmods::installed_files(game) {
            w.protect(&f)?;
            fs::remove_file(game.join(&f))?;
        }
        // Only empty folders are left at this point.
        let gui = rsmods::gui_dir(game);
        if gui.is_dir() {
            fs::remove_dir_all(gui)?;
        }
        if let Some(original) = &original_cache {
            w.protect("cache.psarc")?;
            fs::copy(original, game.join("cache.psarc"))?;
        }
        Ok(())
    })
}

#[tauri::command]
pub fn open_rsmods_gui(app: tauri::AppHandle, ctx: State<'_, Ctx>) -> Result<()> {
    use tauri_plugin_opener::OpenerExt;
    let gui = rsmods::status(&ctx.game_dir()?, None)
        .gui
        .ok_or_else(|| AppError::msg(tr!("rsmods.guiMissing")))?;
    app.opener()
        .open_path(gui, None::<&str>)
        .map_err(|e| AppError::msg(tr!("rsmods.openFailed", error = e)))
}

pub const CUSTOMSFORGE_LABEL: &str = "customsforge";
const AD_FILTER: &str = include_str!("adfilter.js");

/// The filter is a start script of the view, so a change takes effect when the
/// view is recreated; the login survives because cookies are stored on disk.
#[tauri::command]
pub fn set_hide_ads(app: tauri::AppHandle, ctx: State<'_, Ctx>, enabled: bool) -> Result<()> {
    let mut s = ctx.state();
    s.hide_ads = enabled;
    ctx.save_state(s)?;
    if let Some(view) = customsforge_view(&app) {
        view.close().map_err(|e| AppError::msg(e.to_string()))?;
    }
    Ok(())
}
const CUSTOMSFORGE_URL: &str = "https://ignition4.customsforge.com/";

#[derive(Clone, Serialize)]
pub struct DownloadReport {
    file_name: String,
    result: Option<cdlc::ImportResult>,
    error: Option<String>,
}

/// Catches downloads from the CustomsForge view and installs them as songs.
fn handle_download(webview: tauri::Webview, event: tauri::webview::DownloadEvent<'_>) -> bool {
    use tauri::webview::DownloadEvent;
    use tauri::{Emitter, Manager};

    let app = webview.app_handle().clone();
    match &event {
        DownloadEvent::Requested { url, destination } => eprintln!("[customsforge] download requested {url} -> {}", destination.display()),
        DownloadEvent::Finished { url, path, success } => eprintln!("[customsforge] download finished {url} {path:?} success={success}"),
        _ => {}
    }
    match event {
        DownloadEvent::Requested { destination, .. } => {
            let ctx = app.state::<Ctx>();
            let dir = ctx.data_dir.join("downloads");
            let _ = fs::create_dir_all(&dir);
            let name = destination.file_name().map(|n| n.to_owned()).unwrap_or_else(|| "download".into());
            let _ = app.emit_to("main", "cdlc-download-started", name.to_string_lossy().into_owned());
            *destination = dir.join(name);
        }
        DownloadEvent::Finished { url, path, success } => {
            let Some(path) = path else { return true };
            let file_name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let report = if !success {
                DownloadReport { file_name, result: None, error: Some(tr!("cdlc.downloadCanceled")) }
            } else {
                let ctx = app.state::<Ctx>();
                match ctx.game_dir().and_then(|g| cdlc::import(&g, &ctx.data_dir, &[path.clone()])) {
                    Ok(result) => {
                        let _ = fs::remove_file(&path);
                        app.state::<CfDownloads>().record(&ctx.data_dir, &result.imported, url.host_str());
                        DownloadReport { file_name, result: Some(result), error: None }
                    }
                    Err(e) => DownloadReport { file_name, result: None, error: Some(e.to_string()) },
                }
            };
            let _ = app.emit_to("main", "cdlc-download", report);
        }
        _ => {}
    }
    true
}

fn customsforge_view(app: &tauri::AppHandle) -> Option<tauri::Webview> {
    use tauri::Manager;
    app.get_webview(CUSTOMSFORGE_LABEL)
}

/// Shows CustomsForge as a layer inside the main window, at the given area in
/// CSS pixels. It is a separate webview: the page gets no access to the app's
/// commands (capabilities only cover the "main" webview).
#[tauri::command]
pub async fn open_customsforge(app: tauri::AppHandle, x: f64, y: f64, width: f64, height: f64) -> Result<()> {
    use tauri::{LogicalPosition, LogicalSize};

    let fail = |e: tauri::Error| AppError::msg(tr!("customsforge.openFailed", error = e));
    if let Some(view) = customsforge_view(&app) {
        view.set_position(LogicalPosition::new(x, y)).map_err(fail)?;
        view.set_size(LogicalSize::new(width, height)).map_err(fail)?;
        view.show().map_err(fail)?;
        view.set_focus().ok();
        return Ok(());
    }
    create_customsforge_view(&app, x, y, width, height)?;
    Ok(())
}

/// Creates the CustomsForge view; also used hidden for the song update check.
fn create_customsforge_view(app: &tauri::AppHandle, x: f64, y: f64, width: f64, height: f64) -> Result<tauri::Webview> {
    use tauri::{LogicalPosition, LogicalSize, Manager, WebviewBuilder, WebviewUrl};

    let fail = |e: tauri::Error| AppError::msg(tr!("customsforge.openFailed", error = e));
    let window = app.get_window("main").ok_or_else(|| AppError::msg(tr!("customsforge.mainWindowMissing")))?;
    let url = CUSTOMSFORGE_URL.parse().expect("static URL");
    // Download buttons on CustomsForge open the file host (Google Drive, Mega …)
    // in a new tab. There are no tabs here, so the link opens in the same view,
    // where the download can be caught; "←" leads back to CustomsForge.
    let nav_app = app.clone();
    let drive_app = app.clone();
    let mut builder = WebviewBuilder::new(CUSTOMSFORGE_LABEL, WebviewUrl::External(url))
        .on_download(handle_download)
        .on_navigation(move |url| {
            drive_app.state::<CfDownloads>().note_request(url);
            let Some(direct) = cdlc::drive_direct_download(url) else { return true };
            eprintln!("[customsforge] google drive -> {direct}");
            if let Some(view) = customsforge_view(&drive_app) {
                tauri::async_runtime::spawn(async move {
                    let _ = view.navigate(direct);
                });
            }
            false
        })
        .on_page_load(|webview, payload| {
            use tauri::{Emitter, Manager};
            use tauri::webview::PageLoadEvent;
            let state = match payload.event() {
                PageLoadEvent::Started => "started",
                PageLoadEvent::Finished => "finished",
            };
            eprintln!("[customsforge] page {state} {}", payload.url());
            let _ = webview.app_handle().emit_to("main", "customsforge-page", (state, payload.url().to_string()));
            if matches!(payload.event(), PageLoadEvent::Finished) && sources::is_customsforge_url(payload.url()) {
                webview.app_handle().state::<UpdateCheck>().trigger(&webview);
            }
        })
        .on_document_title_changed(|webview, title| {
            use tauri::Manager;
            if let Some(report) = title.strip_prefix(UPDATE_CHECK_PREFIX) {
                let app = webview.app_handle();
                app.state::<UpdateCheck>().handle_report(app, report);
            }
        })
        .initialization_script(UPDATE_CHECK_JS)
        .on_new_window(move |url, _features| {
            eprintln!("[customsforge] new window {url}");
            nav_app.state::<CfDownloads>().note_request(&url);
            if matches!(url.scheme(), "http" | "https") {
                if let Some(view) = customsforge_view(&nav_app) {
                    tauri::async_runtime::spawn(async move {
                        let _ = view.navigate(url);
                    });
                }
            }
            tauri::webview::NewWindowResponse::Deny
        });
    if app.state::<Ctx>().state().hide_ads {
        builder = builder.initialization_script(AD_FILTER);
    }
    window
        .add_child(builder, LogicalPosition::new(x, y), LogicalSize::new(width, height))
        .map_err(fail)
}

const UPDATE_CHECK_JS: &str = include_str!("updatecheck.js");
const UPDATE_CHECK_PREFIX: &str = "GSCHK:";

/// A running song update check: the ids sent to the page and whether the
/// page has been asked yet (it can only be asked once it has loaded).
struct CheckRun {
    ids: Vec<String>,
    triggered: bool,
    done: usize,
}

/// State of the song update check, see `versions.rs` and `updatecheck.js`.
#[derive(Default)]
pub struct UpdateCheck {
    run: Mutex<Option<CheckRun>>,
}

#[derive(Clone, Serialize)]
struct CheckEvent {
    /// "progress", "done", "login", "limited" or "error".
    state: &'static str,
    done: usize,
    total: usize,
}

#[derive(serde::Deserialize)]
struct CheckReport {
    #[serde(rename = "type")]
    kind: String,
    id: Option<String>,
    ok: Option<bool>,
    version: Option<String>,
    revised: Option<String>,
}

impl UpdateCheck {
    fn lock(&self) -> MutexGuard<'_, Option<CheckRun>> {
        self.run.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn running(&self) -> bool {
        self.lock().is_some()
    }

    /// Asks the page to start, once it is loaded on CustomsForge.
    fn trigger(&self, view: &tauri::Webview) {
        let mut run = self.lock();
        let Some(run) = run.as_mut().filter(|r| !r.triggered) else { return };
        let ids = serde_json::to_string(&run.ids).unwrap_or_else(|_| "[]".into());
        if view.eval(format!("window.__gsCheckSongs && window.__gsCheckSongs({ids})")).is_ok() {
            run.triggered = true;
            eprintln!("[update-check] started for {} song(s)", run.ids.len());
        }
    }

    /// Handles one message the page put into its title.
    fn handle_report(&self, app: &tauri::AppHandle, raw: &str) {
        use tauri::{Emitter, Manager};
        let Ok(report) = serde_json::from_str::<CheckReport>(raw) else { return };
        let mut guard = self.lock();
        let Some(run) = guard.as_mut() else { return };
        let total = run.ids.len();
        let finish = |state: &'static str, done: usize| {
            let _ = app.emit_to("main", "cdlc-update-check", CheckEvent { state, done, total });
        };
        match report.kind.as_str() {
            "song" => {
                // Only ids GuitarStar sent are accepted; values are untrusted text.
                let Some(id) = report.id.filter(|id| run.ids.contains(id)) else { return };
                run.done += 1;
                if report.ok == Some(true) {
                    let data_dir = &app.state::<Ctx>().data_dir;
                    let (version, revised) =
                        versions::validate_report(report.version.as_deref(), report.revised.as_deref());
                    let mut store = VersionStore::load(data_dir);
                    store.record(&id, version, revised, now());
                    if let Err(e) = store.save(data_dir) {
                        eprintln!("[update-check] could not save versions: {e}");
                    }
                }
                finish("progress", run.done);
            }
            kind @ ("done" | "login" | "limited" | "error") => {
                let state = match kind {
                    "done" => "done",
                    "login" => "login",
                    "limited" => "limited",
                    _ => "error",
                };
                eprintln!("[update-check] {state} after {} of {total}", run.done);
                let done = run.done;
                *guard = None;
                finish(state, done);
            }
            _ => {}
        }
    }
}

#[derive(Serialize)]
pub struct CheckStart {
    started: bool,
    /// Why nothing was started: "none" (no songs with a known CustomsForge id),
    /// "running", "auto_done", "manual_limit" or "all_checked".
    reason: Option<&'static str>,
    count: usize,
    manual_left: u32,
}

/// Starts a song update check. `manual` counts against the three manual
/// checks per day; otherwise it is the automatic check, at most once a day.
#[tauri::command]
pub async fn cdlc_check_updates(
    app: tauri::AppHandle,
    ctx: State<'_, Ctx>,
    cf: State<'_, CfDownloads>,
    check: State<'_, UpdateCheck>,
    manual: bool,
) -> Result<CheckStart> {
    let game = ctx.game_dir()?;
    let now = now();
    let mut store = VersionStore::load(&ctx.data_dir);
    let refuse = |reason, store: &VersionStore| CheckStart {
        started: false,
        reason: Some(reason),
        count: 0,
        manual_left: store.manual_left(now),
    };
    if check.running() {
        return Ok(refuse("running", &store));
    }
    let installed: Vec<String> = cdlc::list(&game, &ctx.data_dir)?.into_iter().map(|s| s.file_name).collect();
    let ids: Vec<String> = {
        let _guard = cf.lock_sources();
        let sources = Sources::load(&ctx.data_dir);
        installed.iter().filter_map(|f| sources.0.get(f).map(|s| s.cf_id.clone())).collect()
    };
    if ids.is_empty() {
        return Ok(refuse("none", &store));
    }
    if let Err(refusal) = store.start_run(manual, now) {
        let reason = match refusal {
            versions::Refusal::AutoDoneToday => "auto_done",
            versions::Refusal::ManualLimit => "manual_limit",
        };
        return Ok(refuse(reason, &store));
    }
    let selected = store.select(ids.iter().map(String::as_str), manual, now);
    store.save(&ctx.data_dir)?;
    if selected.is_empty() {
        return Ok(refuse("all_checked", &store));
    }
    let count = selected.len();
    *check.lock() = Some(CheckRun { ids: selected, triggered: false, done: 0 });
    match customsforge_view(&app) {
        // An existing view is already loaded; ask it right away.
        Some(view) => check.trigger(&view),
        // Otherwise a hidden view is created; it starts the check once loaded.
        None => {
            let view = create_customsforge_view(&app, 0.0, 0.0, 1.0, 1.0)?;
            view.hide().ok();
        }
    }
    Ok(CheckStart { started: true, reason: None, count, manual_left: store.manual_left(now) })
}

#[derive(Serialize)]
pub struct SongUpdate {
    cf_id: String,
    version: Option<String>,
    revised: Option<String>,
}

#[derive(Serialize)]
pub struct UpdateStatus {
    /// File name → newer version on CustomsForge.
    updates: BTreeMap<String, SongUpdate>,
    manual_left: u32,
    last_check: Option<u64>,
    running: bool,
}

#[tauri::command]
pub fn cdlc_update_status(
    ctx: State<'_, Ctx>,
    cf: State<'_, CfDownloads>,
    check: State<'_, UpdateCheck>,
) -> Result<UpdateStatus> {
    let game = ctx.game_dir()?;
    let store = VersionStore::load(&ctx.data_dir);
    let sources = {
        let _guard = cf.lock_sources();
        Sources::load(&ctx.data_dir)
    };
    let mut updates = BTreeMap::new();
    for song in cdlc::list(&game, &ctx.data_dir)? {
        let Some(source) = sources.0.get(&song.file_name) else { continue };
        let Some(remote) = store.songs.get(&source.cf_id) else { continue };
        if versions::update_available(song.version.as_deref(), Some(source.downloaded_at), remote) {
            updates.insert(
                song.file_name,
                SongUpdate { cf_id: source.cf_id.clone(), version: remote.version.clone(), revised: remote.revised.clone() },
            );
        }
    }
    Ok(UpdateStatus {
        updates,
        manual_left: store.manual_left(now()),
        last_check: store.songs.values().map(|v| v.checked_at).max(),
        running: check.running(),
    })
}

#[tauri::command]
pub fn customsforge_bounds(app: tauri::AppHandle, x: f64, y: f64, width: f64, height: f64) -> Result<()> {
    use tauri::{LogicalPosition, LogicalSize};
    if let Some(view) = customsforge_view(&app) {
        let fail = |e: tauri::Error| AppError::msg(e.to_string());
        view.set_position(LogicalPosition::new(x, y)).map_err(fail)?;
        view.set_size(LogicalSize::new(width, height)).map_err(fail)?;
    }
    Ok(())
}

/// Hides the layer but keeps the page (and the login) for the next time.
#[tauri::command]
pub fn hide_customsforge(app: tauri::AppHandle) -> Result<()> {
    if let Some(view) = customsforge_view(&app) {
        view.hide().map_err(|e| AppError::msg(e.to_string()))?;
    }
    Ok(())
}

#[tauri::command]
pub fn customsforge_navigate(app: tauri::AppHandle, action: String) -> Result<()> {
    let script = match action.as_str() {
        "back" => "history.back()",
        "forward" => "history.forward()",
        "reload" => "location.reload()",
        "home" => "location.href = 'https://ignition4.customsforge.com/'",
        _ => return Err(AppError::msg(tr!("customsforge.unknownAction"))),
    };
    if let Some(view) = customsforge_view(&app) {
        view.eval(script).map_err(|e| AppError::msg(e.to_string()))?;
    }
    Ok(())
}

/// Opens a page of CustomsForge's song site (e.g. a song's page) in the
/// embedded view; other addresses are refused.
#[tauri::command]
pub fn customsforge_open_url(app: tauri::AppHandle, url: String) -> Result<()> {
    let url: reqwest::Url = url.parse().map_err(|_| AppError::msg(tr!("customsforge.invalidAddress")))?;
    if !sources::is_customsforge_url(&url) {
        return Err(AppError::msg(tr!("customsforge.foreignPage")));
    }
    let view = customsforge_view(&app).ok_or_else(|| AppError::msg(tr!("customsforge.notOpen")))?;
    view.navigate(url).map_err(|e| AppError::msg(e.to_string()))
}

#[derive(Serialize)]
pub struct MonitorSize {
    name: Option<String>,
    width: u32,
    height: u32,
    scale_factor: f64,
}

#[tauri::command]
pub fn monitor_sizes(window: tauri::Window) -> Vec<MonitorSize> {
    window
        .available_monitors()
        .unwrap_or_default()
        .into_iter()
        .map(|m| MonitorSize {
            name: m.name().cloned(),
            width: m.size().width,
            height: m.size().height,
            scale_factor: m.scale_factor(),
        })
        .collect()
}
