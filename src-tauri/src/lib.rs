pub mod models;
pub mod store;
pub mod commands;

use std::env;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .setup(|app| {
            // Configurações da Tray
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
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Configuração e Eventos da Janela
            if let Some(window) = app.get_webview_window("main") {
                let args: Vec<String> = env::args().collect();
                if !args.iter().any(|arg| arg == "--autostart") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }

                let w_clone1 = window.clone();
                let w_clone2 = window.clone();
                window.on_window_event(move |event| match event {
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close();
                        let _ = w_clone1.hide();
                    }
                    tauri::WindowEvent::Resized(_) => {
                        if w_clone2.is_minimized().unwrap_or(false) {
                            let _ = w_clone2.hide();
                        }
                    }
                    _ => {}
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::listar_macros,
            commands::salvar_macro,
            commands::deletar_macro,
            commands::alternar_autostart_admin,
            commands::verificar_autostart_admin,
            commands::executar_macro_agora
        ])
        .run(tauri::generate_context!())
        .expect("Erro ao executar aplicação Tauri");
}