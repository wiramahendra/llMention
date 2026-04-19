import type React from "react";

export const card: React.CSSProperties = {
  background: "#161b22",
  border: "1px solid #21262d",
  borderRadius: 8,
  padding: "20px",
  marginBottom: 16,
};

export const input: React.CSSProperties = {
  background: "#0d1117",
  border: "1px solid #30363d",
  borderRadius: 6,
  color: "#e6edf3",
  padding: "8px 12px",
  fontSize: 13,
  fontFamily: "inherit",
  outline: "none",
  width: "100%",
};

export const btn: React.CSSProperties = {
  background: "#238636",
  color: "#fff",
  border: "1px solid #2ea043",
  borderRadius: 6,
  padding: "8px 16px",
  fontSize: 13,
  fontFamily: "inherit",
  cursor: "pointer",
  whiteSpace: "nowrap",
};

export const btnSecondary: React.CSSProperties = {
  ...btn,
  background: "#21262d",
  borderColor: "#30363d",
  color: "#e6edf3",
};

export const heading: React.CSSProperties = {
  fontSize: 22,
  fontWeight: 700,
  marginBottom: 6,
  color: "#e6edf3",
};

export const muted: React.CSSProperties = {
  fontSize: 13,
  color: "#8b949e",
  marginBottom: 20,
};

export const tag = (color: string): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: 12,
  fontSize: 11,
  fontWeight: 600,
  background: `${color}22`,
  color: color,
});

export const pre: React.CSSProperties = {
  background: "#0d1117",
  border: "1px solid #21262d",
  borderRadius: 6,
  padding: 16,
  fontSize: 12,
  lineHeight: 1.6,
  overflowX: "auto",
  whiteSpace: "pre-wrap",
  color: "#e6edf3",
};
