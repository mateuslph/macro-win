use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Serialize, Deserialize, Clone)]
struct MacroItem {
    id: String,
    caminho_bat: String,
    horario: String,
    ao_iniciar: bool,
    deletada_em: Option<u64>,
}

fn db_path(app: &tauri::AppHandle) -> PathBuf {
    let path = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let _ = fs::create_dir_all(&path);
    path.join("macros.json")
}

fn ler_db(app: &tauri::AppHandle) -> Vec<MacroItem> {
    if let Ok(data) = fs::read_to_string(db_path(app)) {
        let mut macros: Vec<MacroItem> = serde_json::from_str(&data).unwrap_or_default();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let trinta_dias = 30 * 24 * 60 * 60;
        // Retém apenas macros ativas ou na lixeira há menos de 30 dias
        macros.retain(|m| m.deletada_em.map_or(true, |d| now - d < trinta_dias));
        let _ = fs::write(db_path(app), serde_json::to_string(&macros).unwrap());
        macros
    } else {
        vec![]
    }
}

#[tauri::command]
fn listar_macros(app: tauri::AppHandle) -> Result<Vec<MacroItem>, String> {
    Ok(ler_db(&app))
}

#[tauri::command]
fn salvar_macro(app: tauri::AppHandle, id: String, caminho_bat: String, horario: String, ao_iniciar: bool) -> Result<(), String> {
    let mut macros = ler_db(&app);
    macros.retain(|m| m.id != id);

    // 1. Cria o VBScript "Ocultador" se ele ainda não existir na pasta do app
    let dir_app = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let _ = fs::create_dir_all(&dir_app);
    let vbs_path = dir_app.join("oculto.vbs");
    
    if !vbs_path.exists() {
        // O zero (0) no final do comando VBS significa janela 100% oculta
        let vbs_code = "CreateObject(\"WScript.Shell\").Run chr(34) & WScript.Arguments(0) & chr(34), 0, False";
        let _ = fs::write(&vbs_path, vbs_code);
    }

    // 2. Configura o Agendador para rodar o WScript invisível, que por sua vez roda o seu .bat
    let comando_invisivel = format!("wscript.exe //B \"{}\" \"{}\"", vbs_path.display(), caminho_bat);

    let mut cmd = Command::new("schtasks");
    cmd.args(["/Create", "/F", "/TN", &format!("MacroWin\\{}", id)]);
    cmd.args(["/TR", &comando_invisivel]); // <- Usa o VBS aqui
    
    if ao_iniciar {
        cmd.args(["/SC", "ONLOGON"]);
    } else {
        cmd.args(["/SC", "ONCE", "/ST", &horario]);
    }
    
    #[cfg(target_os = "windows")] { cmd.creation_flags(0x08000000); }
    let _ = cmd.output().map_err(|e| e.to_string())?;

    macros.push(MacroItem { id, caminho_bat, horario, ao_iniciar, deletada_em: None });
    fs::write(db_path(&app), serde_json::to_string(&macros).unwrap()).unwrap();
    Ok(())
}

#[tauri::command]
fn deletar_macro(app: tauri::AppHandle, id: String, definitivo: bool) -> Result<(), String> {
    let mut cmd = Command::new("schtasks");
    cmd.args(["/Delete", "/F", "/TN", &format!("MacroWin\\{}", id)]);
    #[cfg(target_os = "windows")] { cmd.creation_flags(0x08000000); }
    let _ = cmd.output();

    let mut macros = ler_db(&app);
    if definitivo {
        macros.retain(|m| m.id != id);
    } else if let Some(m) = macros.iter_mut().find(|m| m.id == id) {
        m.deletada_em = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
    }
    fs::write(db_path(&app), serde_json::to_string(&macros).unwrap()).unwrap();
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--autostart"])))
        .setup(|app| {
            let mostrar_i = MenuItem::with_id(app, "mostrar", "Mostrar Agendador", true, None::<&str>)?;
            let sair_i = MenuItem::with_id(app, "sair", "Encerrar Completamente", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&mostrar_i, &sair_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "sair" => app.exit(0),
                    "mostrar" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show(); let _ = window.unminimize(); let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show(); let _ = window.unminimize(); let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            if let Some(window) = app.get_webview_window("main") {
                let args: Vec<String> = std::env::args().collect();
                if !args.contains(&"--autostart".to_string()) {
                    let _ = window.show();
                    let _ = window.set_focus();
                }

                let w_clone1 = window.clone();
                let w_clone2 = window.clone();
                
                window.on_window_event(move |event| match event {
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close(); // Impede o fechamento total
                        let _ = w_clone1.hide(); // Oculta na bandeja
                    }
                    tauri::WindowEvent::Resized(_) => {
                        if w_clone2.is_minimized().unwrap_or(false) {
                            let _ = w_clone2.hide(); // Oculta na bandeja ao minimizar
                        }
                    }
                    _ => {}
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![listar_macros, salvar_macro, deletar_macro])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}