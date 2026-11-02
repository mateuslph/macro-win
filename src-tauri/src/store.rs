use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::Manager;
use crate::models::{AutostartConfig, MacroItem};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

pub fn diretorio_app(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let path = app.path().app_data_dir().map_err(|e| format!("Erro ao obter diretório: {}", e))?;
    fs::create_dir_all(&path).map_err(|e| format!("Erro ao criar diretório: {}", e))?;
    Ok(path)
}

fn db_path(app: &tauri::AppHandle) -> PathBuf {
    diretorio_app(app).unwrap_or_else(|_| PathBuf::from(".")).join("macros.json")
}

fn autostart_config_path(app: &tauri::AppHandle) -> PathBuf {
    diretorio_app(app).unwrap_or_else(|_| PathBuf::from(".")).join("autostart_config.json")
}

pub fn ler_db(app: &tauri::AppHandle) -> Vec<MacroItem> {
    let path = db_path(app);
    let data = match fs::read_to_string(&path) {
        Ok(data) => data,
        Err(_) => return Vec::new(),
    };

    let mut macros: Vec<MacroItem> = serde_json::from_str(&data).unwrap_or_default();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let trinta_dias = 30 * 24 * 60 * 60;

    macros.retain(|m| m.deletada_em.map_or(true, |data| now.saturating_sub(data) < trinta_dias));

    if let Ok(json) = serde_json::to_string_pretty(&macros) {
        let _ = fs::write(path, json);
    }
    macros
}

pub fn salvar_db(app: &tauri::AppHandle, macros: &[MacroItem]) -> Result<(), String> {
    let json = serde_json::to_string_pretty(macros).map_err(|e| format!("Erro ao serializar: {}", e))?;
    fs::write(db_path(app), json).map_err(|e| format!("Erro ao salvar: {}", e))?;
    Ok(())
}

pub fn ler_autostart_config(app: &tauri::AppHandle) -> AutostartConfig {
    match fs::read_to_string(autostart_config_path(app)) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => AutostartConfig::default(),
    }
}

pub fn salvar_autostart_config(app: &tauri::AppHandle, ativar: bool) -> Result<(), String> {
    let config = AutostartConfig { iniciar_com_windows_admin: ativar };
    let json = serde_json::to_string_pretty(&config).map_err(|e| format!("Erro ao serializar: {}", e))?;
    fs::write(autostart_config_path(app), json).map_err(|e| format!("Erro ao salvar: {}", e))?;
    Ok(())
}

pub fn executar_comando_oculto(programa: &str, argumentos: &[&str]) -> Result<Output, String> {
    let mut cmd = Command::new(programa);
    cmd.args(argumentos);

    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(0x08000000);
    }

    cmd.output().map_err(|e| format!("Erro ao executar {}: {}", programa, e))
}

pub fn verificar_tarefa(nome: &str) -> Result<bool, String> {
    let output = executar_comando_oculto("schtasks", &["/Query", "/TN", nome])?;
    Ok(output.status.success())
}

#[cfg(target_os = "windows")]
pub fn executar_como_administrador() -> bool {
    matches!(executar_comando_oculto("net", &["session"]), Ok(ref r) if r.status.success())
}

#[cfg(not(target_os = "windows"))]
pub fn executar_como_administrador() -> bool {
    false
}