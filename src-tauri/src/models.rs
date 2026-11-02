use serde::{Deserialize, Serialize};

pub const AUTOSTART_TASK: &str = "MacroWin_SystemAutostart";
pub const MACRO_TASK_PREFIX: &str = "MacroWin";

#[derive(Serialize, Deserialize, Clone)]
pub struct MacroItem {
    pub id: String,
    pub caminho_bat: String,
    pub horario: String,
    pub ao_iniciar: bool,
    pub deletada_em: Option<u64>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct AutostartConfig {
    pub iniciar_com_windows_admin: bool,
}