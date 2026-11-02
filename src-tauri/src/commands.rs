use std::{env, fs, process::Command, time::{SystemTime, UNIX_EPOCH}};
use tauri_plugin_notification::NotificationExt;
use crate::models::{MacroItem, AUTOSTART_TASK, MACRO_TASK_PREFIX};
use crate::store::{
    diretorio_app, executar_comando_oculto, ler_db, salvar_db,
    salvar_autostart_config, ler_autostart_config, verificar_tarefa, executar_como_administrador
};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[tauri::command]
pub fn listar_macros(app: tauri::AppHandle) -> Result<Vec<MacroItem>, String> {
    Ok(ler_db(&app))
}

#[tauri::command]
pub fn salvar_macro(
    app: tauri::AppHandle, id: String, caminho_bat: String, horario: String, ao_iniciar: bool
) -> Result<(), String> {
    let mut macros = ler_db(&app);
    macros.retain(|m| m.id != id);

    let dir_app = diretorio_app(&app)?;
    let vbs_path = dir_app.join("oculto.vbs");

    if !vbs_path.exists() {
        let vbs_code = "CreateObject(\"WScript.Shell\").Run chr(34) & WScript.Arguments(0) & chr(34), 0, False";
        fs::write(&vbs_path, vbs_code).map_err(|e| format!("Erro ao criar VBS: {}", e))?;
    }

    let vbs_str = vbs_path.display().to_string().replace("\\\\?\\", "");
    let bat_str = caminho_bat.replace("\\\\?\\", "");
    let comando_invisivel = format!("wscript.exe //B \"{}\" \"{}\"", vbs_str, bat_str);
    let nome_tarefa = format!("{}\\{}", MACRO_TASK_PREFIX, id);

    let mut cmd = Command::new("schtasks");
    cmd.args(["/Create", "/F", "/TN", &nome_tarefa, "/TR", &comando_invisivel]);

    if ao_iniciar {
        cmd.args(["/SC", "ONLOGON"]);
    } else {
        cmd.args(["/SC", "ONCE", "/ST", &horario]);
    }

    #[cfg(target_os = "windows")]
    { cmd.creation_flags(0x08000000); }

    let output = cmd.output().map_err(|e| format!("Erro no schtasks: {}", e))?;
    if !output.status.success() {
        return Err(format!("Falha ao salvar macro:\n{}", String::from_utf8_lossy(&output.stderr)));
    }

    macros.push(MacroItem { id, caminho_bat, horario, ao_iniciar, deletada_em: None });
    salvar_db(&app, &macros)?;
    Ok(())
}

#[tauri::command]
pub fn deletar_macro(app: tauri::AppHandle, id: String, definitivo: bool) -> Result<(), String> {
    let nome_tarefa = format!("{}\\{}", MACRO_TASK_PREFIX, id);
    let _ = executar_comando_oculto("schtasks", &["/Delete", "/F", "/TN", &nome_tarefa]);

    let mut macros = ler_db(&app);
    if definitivo {
        macros.retain(|m| m.id != id);
    } else if let Some(m) = macros.iter_mut().find(|m| m.id == id) {
        m.deletada_em = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs());
    }

    salvar_db(&app, &macros)?;
    Ok(())
}

#[tauri::command]
pub fn alternar_autostart_admin(app: tauri::AppHandle, ativar: bool) -> Result<(), String> {
    if ativar {
        if !executar_como_administrador() {
            return Err("Permissão administrativa necessária.".to_string());
        }
        let exe_path = env::current_exe().map_err(|e| format!("Erro no executável: {}", e))?;
        let comando = format!("\"{}\" --autostart", exe_path.display().to_string().replace("\\\\?\\", ""));

        let output = executar_comando_oculto("schtasks", &[
            "/Create", "/F", "/TN", AUTOSTART_TASK, "/TR", &comando, "/SC", "ONLOGON", "/RU", "SYSTEM", "/RL", "HIGHEST"
        ])?;

        if !output.status.success() {
            return Err(format!("Falha ao criar tarefa:\n{}", String::from_utf8_lossy(&output.stderr)));
        }
    } else {
        let output = executar_comando_oculto("schtasks", &["/Delete", "/F", "/TN", AUTOSTART_TASK])?;
        if !output.status.success() {
            if let Ok(true) = verificar_tarefa(AUTOSTART_TASK) {
                return Err(format!("Falha ao remover tarefa:\n{}", String::from_utf8_lossy(&output.stderr)));
            }
        }
    }
    salvar_autostart_config(&app, ativar)?;
    Ok(())
}

#[tauri::command]
pub fn verificar_autostart_admin(app: tauri::AppHandle) -> bool {
    match verificar_tarefa(AUTOSTART_TASK) {
        Ok(ativo) => {
            let _ = salvar_autostart_config(&app, ativo);
            ativo
        }
        Err(_) => ler_autostart_config(&app).iniciar_com_windows_admin
    }
}

#[tauri::command]
pub fn executar_macro_agora(app: tauri::AppHandle, caminho_bat: String) -> Result<(), String> {
    let bat_str = caminho_bat.replace("\\\\?\\", "");
    let output = executar_comando_oculto("cmd", &["/C", &bat_str])?;

    if !output.status.success() {
        return Err(format!("Falha ao executar macro. Código: {:?}", output.status.code()));
    }

    let _ = app.notification().builder().title("MacroWin").body("Macro testada com sucesso!").show();
    Ok(())
}