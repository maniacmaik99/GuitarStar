//! Commands called from the UI. Every operation that changes the game folder
//! runs under `Ctx::op`, so two clicks can never modify files at the same time.

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
use guitarstar_core::snapshot::{SnapshotStore, SnapshotWriter};
use guitarstar_core::state::{AppState, InstalledComponent};
use guitarstar_core::{display, reqwest, steam};

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
            .ok_or_else(|| AppError::msg("Bitte zuerst den Rocksmith-Spielordner auswählen."))
    }

    fn snapshots(&self) -> SnapshotStore {
        SnapshotStore::new(self.data_dir.join("snapshots"))
    }

    fn profiles_dir(&self) -> PathBuf {
        self.data_dir.join("profiles")
    }

    /// Runs `change` inside a snapshot; on error the partial change is rolled back.
    fn change_game_files(
        &self,
        label: &str,
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
    label: String,
    file_count: usize,
}

#[derive(Serialize)]
pub struct ComponentStatus {
    id: &'static str,
    name: &'static str,
    description: &'static str,
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
                name: def.name,
                description: def.description,
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
        .map(|s| SnapshotSummary { file_count: s.files.len(), id: s.id, created: s.created, label: s.label })
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
    ctx.change_game_files("Spieleinstellungen geändert", |game, w| {
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
pub fn list_asio_drivers() -> Vec<AsioDriver> {
    audio::asio_drivers()
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
        return Err(AppError::msg("Bitte zuerst RS_ASIO installieren."));
    }
    let template = rs_asio_template(&ctx);
    ctx.change_game_files("Audio-Einstellungen geändert", |game, w| {
        write_rs_asio_ini(game, &template, &config, w)?;
        // RS_ASIO's recommended game settings; only possible once the game created its ini.
        if game.join(game::ROCKSMITH_INI).is_file() {
            let s = GameSettings {
                exclusive_mode: Some(true),
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
            .ok_or_else(|| AppError::msg(format!("{} {version} wurde auf GitHub nicht gefunden.", def.name)))?;
        components::download(&ctx.http, &ctx.data_dir, def, &rel).await?;
    }
    let source = components::cache_dir(&ctx.data_dir, def.id, &version);
    let state = ctx.state();
    let label = match state.installed.get(def.id) {
        Some(prev) if prev.version != version => format!("{} {} → {version}", def.name, prev.version),
        _ => format!("{} {version} installiert", def.name),
    };
    let mut files: Vec<String> = def.files.iter().map(|f| f.to_string()).collect();
    ctx.change_game_files(&label, |game, w| {
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
    ctx.change_game_files(&format!("{} entfernt", def.name), |game, w| {
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
    // The selected folder is not part of what gets undone.
    s.game_dir = ctx.state().game_dir;
    if s.dpi_override != ctx.state().dpi_override {
        display::set_dpi_override(&game_exe(&ctx.game_dir()?), s.dpi_override)?;
    }
    ctx.save_state(s)
}

#[tauri::command]
pub async fn set_keep_visible(ctx: State<'_, Ctx>, enabled: bool) -> Result<()> {
    let _op = ctx.op.lock().await;
    let mut s = ctx.state();
    let Some(fix) = s.installed.get_mut(components::DPI_FIX_ID) else {
        return Err(AppError::msg("Bitte zuerst den Skalierungs-Fix installieren."));
    };
    let label = if enabled { "Minimieren bei Fokusverlust aus" } else { "Minimieren bei Fokusverlust an" };
    // Logging stays on while the option is new, so problems can be diagnosed from the log.
    ctx.change_game_files(label, |game, w| game::write_window_config(game, enabled, enabled, w))?;
    if !fix.files.iter().any(|f| f == game::GUITARSTAR_INI) {
        fix.files.push(game::GUITARSTAR_INI.to_string());
    }
    s.keep_visible = enabled;
    ctx.save_state(s)
}

#[tauri::command]
pub async fn set_dpi_override(ctx: State<'_, Ctx>, enabled: bool) -> Result<()> {
    let _op = ctx.op.lock().await;
    let game = ctx.game_dir()?;
    // Recorded as a snapshot without files so it shows up in the history and can be undone.
    let label = if enabled { "Windows-Skalierung für Rocksmith aus" } else { "Windows-Skalierung für Rocksmith an" };
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
    match ctx.snapshots().restore_original(&ctx.game_dir()?)? {
        Some(restored) => apply_restored_state(&ctx, restored),
        None => Ok(()),
    }
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
