import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import "./App.css";

type Macro = { id: string, caminho_bat: string, horario: string, ao_iniciar: boolean, deletada_em: number | null };

function App() {
  const [iniciarComWindows, setIniciarComWindows] = useState(false);
  const [aba, setAba] = useState<"form" | "ativas" | "lixeira">("ativas");
  const [macros, setMacros] = useState<Macro[]>([]);
  const [editId, setEditId] = useState<string | null>(null);
  const [caminhoBat, setCaminhoBat] = useState("");
  const [horario, setHorario] = useState("12:00");
  const [aoIniciar, setAoIniciar] = useState(false);

  const carregarMacros = async () => {
    const data: Macro[] = await invoke("listar_macros");
    setMacros(data);
  };

  useEffect(() => { 
    carregarMacros(); 
    invoke<boolean>("verificar_autostart_admin").then(setIniciarComWindows);
  }, [aba]);

  const handleToggleWindows = async (checked: boolean) => {
    setIniciarComWindows(checked);
    try {
      await invoke("alternar_autostart_admin", { ativar: checked });
    } catch (err) {
      console.error("Erro ao configurar autostart admin:", err);
      setIniciarComWindows(!checked);
    }
  };

  const selecionarArquivo = async () => {
    const selected = await open({ filters: [{ name: 'Scripts', extensions: ['bat', 'cmd'] }] });
    if (typeof selected === 'string') {
      setCaminhoBat(selected);
    }
  };

  const handleSalvar = async () => {
    const id = editId || "Macro_" + Date.now();
    await invoke("salvar_macro", { id, caminhoBat, horario, aoIniciar });
    setEditId(null); setCaminhoBat(""); setAba("ativas");
  };

  const editarMacro = (m: Macro) => {
    setEditId(m.id); setCaminhoBat(m.caminho_bat); setHorario(m.horario); setAoIniciar(m.ao_iniciar); setAba("form");
  };

  const acaoMacro = async (id: string, acao: "lixeira" | "definitivo" | "restaurar", m?: Macro) => {
    if (acao === "restaurar" && m) {
      await invoke("salvar_macro", { id: m.id, caminhoBat: m.caminho_bat, horario: m.horario, aoIniciar: m.ao_iniciar });
    } else {
      await invoke("deletar_macro", { id, definitivo: acao === "definitivo" });
    }
    carregarMacros();
  };

  // NOVA FUNÇÃO: Aciona o backend para testar a macro na hora
  const testarMacro = async (caminhoBat: string) => {
    try {
      await invoke("executar_macro_agora", { caminhoBat });
    } catch (err) {
      console.error("Erro ao testar macro:", err);
      alert("Erro ao executar a macro: " + err);
    }
  };

  const ativas = macros.filter(m => m.deletada_em === null);
  const lixeira = macros.filter(m => m.deletada_em !== null);

  return (
    <main className="container">
      <h1 style={{ marginBottom: "1rem" }}>Agendador de Macros</h1>
      
      <div className="autostart-toggle">
        <input type="checkbox" id="auto" checked={iniciarComWindows} onChange={(e) => handleToggleWindows(e.target.checked)} />
        <label htmlFor="auto">Iniciar Gerenciador oculto com o Windows (Admin)</label>
      </div>

      <div className="tabs">
        <button className={aba === "ativas" ? "active" : ""} onClick={() => setAba("ativas")}>Ativas ({ativas.length})</button>
        <button className={aba === "lixeira" ? "active" : ""} onClick={() => setAba("lixeira")}>Lixeira ({lixeira.length})</button>
        <button className={aba === "form" ? "active" : ""} onClick={() => { setEditId(null); setCaminhoBat(""); setAba("form"); }}>+ Nova</button>
      </div>

      {aba === "form" && (
        <div className="card">
          <h3>{editId ? "Editar Macro" : "Nova Macro"}</h3>
          <button className="btn-secondary" onClick={selecionarArquivo}>📂 {caminhoBat || "Selecionar .bat"}</button>
          <input type="time" value={horario} onChange={(e) => setHorario(e.target.value)} disabled={aoIniciar} />
          <label className="boot-label">
            <input type="checkbox" checked={aoIniciar} onChange={(e) => setAoIniciar(e.target.checked)} /> 
            Agendar para rodar no Boot do sistema
          </label>
          <button className="btn-primary" onClick={handleSalvar} disabled={!caminhoBat}>Salvar Macro</button>
        </div>
      )}

      {aba === "ativas" && ativas.map(m => (
        <div key={m.id} className="card item">
          <p><strong>{m.caminho_bat.split('\\').pop()}</strong> - {m.ao_iniciar ? "No boot" : m.horario}</p>
          <div className="actions">
            <button onClick={() => testarMacro(m.caminho_bat)} title="Testar agora">▶️</button>
            <button onClick={() => editarMacro(m)} title="Editar">✏️</button>
            <button onClick={() => acaoMacro(m.id, "lixeira")} title="Excluir">🗑️</button>
          </div>
        </div>
      ))}

      {aba === "lixeira" && lixeira.map(m => (
        <div key={m.id} className="card item">
          <p><strong>{m.caminho_bat.split('\\').pop()}</strong><br/><small>Exclui em {30 - Math.floor((Date.now() / 1000 - m.deletada_em!) / 86400)} dias</small></p>
          <div className="actions">
            <button onClick={() => acaoMacro(m.id, "restaurar", m)}>♻️ Restaurar</button>
            <button onClick={() => acaoMacro(m.id, "definitivo")}>❌ Destruir</button>
          </div>
        </div>
      ))}
    </main>
  );
}
export default App;