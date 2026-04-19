import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { card, input, btn, btnSecondary, heading, muted } from "./styles";

interface Project {
  domain: string;
  niche: string | null;
  notes: string | null;
  last_audited: string | null;
}

const EXAMPLE_PROJECTS = [
  { domain: "igrisinertial.com", niche: "deterministic edge runtime", notes: "Real-world GEO case study" },
  { domain: "ripgrep.rs", niche: "Rust CLI tool", notes: "Popular open source tool" },
];

export default function ProjectsPanel() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [domain, setDomain] = useState("");
  const [niche, setNiche] = useState("");
  const [notes, setNotes] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function load() {
    try {
      const r = await invoke<Project[]>("list_projects");
      setProjects(r);
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => { load(); }, []);

  async function add() {
    if (!domain.trim()) return;
    setLoading(true);
    try {
      await invoke("add_project", { domain: domain.trim(), niche: niche.trim() || null, notes: notes.trim() || null });
      setDomain(""); setNiche(""); setNotes("");
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function remove(d: string) {
    try {
      await invoke("remove_project", { domain: d });
      await load();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div>
      <h1 style={heading}>Projects</h1>
      <p style={muted}>Save domain + niche pairs for quick re-auditing.</p>

      <div style={card}>
        <div style={{ display: "flex", gap: 10, marginBottom: 10 }}>
          <input style={{ ...input, flex: 2 }} placeholder="Domain *" value={domain} onChange={e => setDomain(e.target.value)} />
          <input style={{ ...input, flex: 1 }} placeholder="Niche" value={niche} onChange={e => setNiche(e.target.value)} />
        </div>
        <div style={{ display: "flex", gap: 10 }}>
          <input style={{ ...input, flex: 1 }} placeholder="Notes (optional)" value={notes} onChange={e => setNotes(e.target.value)} />
          <button style={btn} onClick={add} disabled={loading || !domain.trim()}>
            {loading ? "Saving…" : "Add Project"}
          </button>
        </div>
        {error && <div style={{ color: "#f85149", fontSize: 13, marginTop: 8 }}>Error: {error}</div>}
      </div>

      {projects.length === 0 ? (
        <div style={{ color: "#8b949e", fontSize: 13 }}>
          <div style={{ marginBottom: 8 }}>No projects yet. Add one above.</div>
          <div style={{ fontSize: 11, opacity: 0.7 }}>
            Example: <code>igrisinertial.com</code> · niche: <code>deterministic edge runtime</code>
          </div>
        </div>
      ) : (
        <div style={card}>
          {projects.map((p, i) => (
            <div key={i} style={{
              display: "flex", justifyContent: "space-between", alignItems: "center",
              padding: "10px 0",
              borderBottom: i < projects.length - 1 ? "1px solid #21262d" : "none",
            }}>
              <div>
                <div style={{ fontWeight: 600, fontSize: 14 }}>{p.domain}</div>
                <div style={{ fontSize: 12, color: "#8b949e", marginTop: 2 }}>
                  {p.niche ?? "no niche"} {p.last_audited ? `· last audited ${p.last_audited.slice(0, 10)}` : "· never audited"}
                </div>
              </div>
              <button onClick={() => remove(p.domain)} style={{
                ...btnSecondary, padding: "4px 10px", fontSize: 12,
              }}>Remove</button>
            </div>
          ))}
        </div>
      )}

      {projects.length > 0 && (
        <div style={{ marginTop: 24 }}>
          <div style={{ fontSize: 13, color: "#8b949e", marginBottom: 12 }}>Example projects (for inspiration):</div>
          <div style={card}>
            {EXAMPLE_PROJECTS.map((p, i) => (
              <div key={i} style={{
                padding: "8px 0",
                borderBottom: i < EXAMPLE_PROJECTS.length - 1 ? "1px solid #21262d" : "none",
              }}>
                <div style={{ fontWeight: 600, fontSize: 14, color: "#58a6ff" }}>{p.domain}</div>
                <div style={{ fontSize: 12, color: "#8b949e", marginTop: 2 }}>
                  {p.niche} — {p.notes}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
