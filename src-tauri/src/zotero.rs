//! Zotero-Anbindung (Anfragen aus Rust – kein CORS-/Browser-Block).
//!
//! 1. **Better BibTeX** (empfohlen): JSON-RPC `item.search` / `item.export`
//!    liefert stabile Zitierschlüssel.
//! 2. **Zotero-7-Local-API** (Einstellung „Anderen Anwendungen auf diesem
//!    Computer erlauben, mit Zotero zu kommunizieren“): `/api/users/0/items`.
//!
//! Ausgewählte Einträge werden als BibTeX in die Projekt-.bib übernommen
//! (`core::bibtex::merge`) und mit ihrem **BibTeX-Schlüssel** zitiert.

use serde::Serialize;
use serde_json::{json, Value};
use std::time::Duration;

const BASE: &str = "http://127.0.0.1:23119";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoteroStatus {
    pub running: bool,
    pub better_bibtex: bool,
    pub local_api: bool,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoteroItem {
    /// BBT: Zitierschlüssel; Local-API: Zotero-Item-Key.
    pub id: String,
    /// bbt | local
    pub source: String,
    pub title: String,
    pub authors: String,
    pub year: String,
    pub item_type: String,
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(2))
        .no_proxy()
        .build()
        .map_err(|error| format!("HTTP-Client konnte nicht erstellt werden: {error}"))
}

fn get_text(path: &str) -> Result<(u16, String), String> {
    let response = client()?
        .get(format!("{BASE}{path}"))
        .header("Zotero-Allowed-Request", "1")
        .send()
        .map_err(|_| "Zotero ist nicht erreichbar. Läuft Zotero?".to_string())?;
    let status = response.status().as_u16();
    let text = response
        .text()
        .map_err(|error| format!("Antwort von Zotero unlesbar: {error}"))?;
    Ok((status, text))
}

fn rpc(method: &str, params: Value) -> Result<Value, String> {
    let body = json!({ "jsonrpc": "2.0", "method": method, "params": params, "id": 1 });
    let response = client()?
        .post(format!("{BASE}/better-bibtex/json-rpc"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(body.to_string())
        .send()
        .map_err(|_| "Zotero ist nicht erreichbar. Läuft Zotero?".to_string())?;
    let status = response.status().as_u16();
    let text = response
        .text()
        .map_err(|error| format!("Antwort von Better BibTeX unlesbar: {error}"))?;
    if status != 200 {
        return Err(format!("Better BibTeX antwortete mit HTTP {status}."));
    }
    let value: Value = serde_json::from_str(&text)
        .map_err(|_| "Better BibTeX lieferte keine gültige Antwort.".to_string())?;
    if let Some(error) = value.get("error") {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("unbekannter Fehler");
        return Err(format!("Better BibTeX: {message}"));
    }
    Ok(value.get("result").cloned().unwrap_or(Value::Null))
}

pub fn status() -> ZoteroStatus {
    let running = matches!(get_text("/connector/ping"), Ok((200, _)));
    if !running {
        return ZoteroStatus {
            running: false,
            better_bibtex: false,
            local_api: false,
            message: "Zotero läuft nicht oder ist nicht erreichbar (Port 23119).".into(),
        };
    }
    let better_bibtex = matches!(
        get_text("/better-bibtex/cayw?probe=true"),
        Ok((200, text)) if text.trim() == "ready"
    );
    let local_api = matches!(
        get_text("/api/users/0/items/top?limit=1&format=json"),
        Ok((200, _))
    );
    let message = if better_bibtex {
        "Zotero mit Better BibTeX verbunden.".to_string()
    } else if local_api {
        "Zotero verbunden (lokale API). Für stabile Zitierschlüssel wird Better BibTeX empfohlen."
            .to_string()
    } else {
        "Zotero läuft, aber weder Better BibTeX noch die lokale API sind verfügbar. In Zotero 7 unter Einstellungen → Erweitert „Anderen Anwendungen auf diesem Computer erlauben, mit Zotero zu kommunizieren“ aktivieren.".to_string()
    };
    ZoteroStatus {
        running,
        better_bibtex,
        local_api,
        message,
    }
}

fn creators_text(creators: &[Value]) -> String {
    let names: Vec<String> = creators
        .iter()
        .filter_map(|creator| {
            creator
                .get("lastName")
                .or_else(|| creator.get("family"))
                .or_else(|| creator.get("name"))
                .or_else(|| creator.get("literal"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect();
    match names.len() {
        0 => String::new(),
        1 | 2 => names.join(" und "),
        _ => format!("{} u. a.", names[0]),
    }
}

fn year_of(text: &str) -> String {
    text.chars()
        .collect::<Vec<_>>()
        .windows(4)
        .find(|window| window.iter().all(char::is_ascii_digit))
        .map(|window| window.iter().collect())
        .unwrap_or_default()
}

pub fn search(query: &str) -> Result<Vec<ZoteroItem>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let state = status();
    if state.better_bibtex {
        let result = rpc("item.search", json!([query]))?;
        let items = result.as_array().cloned().unwrap_or_default();
        return Ok(items
            .iter()
            .take(50)
            .filter_map(|item| {
                let id = item
                    .get("citationKey")
                    .or_else(|| item.get("citekey"))
                    .or_else(|| item.get("citation-key"))
                    .and_then(Value::as_str)?
                    .to_string();
                let year = item
                    .get("issued")
                    .and_then(|issued| issued.get("date-parts"))
                    .and_then(|parts| parts.get(0))
                    .and_then(|first| first.get(0))
                    .map(|year| year.to_string().trim_matches('"').to_string())
                    .unwrap_or_default();
                Some(ZoteroItem {
                    id,
                    source: "bbt".into(),
                    title: item
                        .get("title")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .into(),
                    authors: creators_text(
                        item.get("author")
                            .and_then(Value::as_array)
                            .map(Vec::as_slice)
                            .unwrap_or(&[]),
                    ),
                    year,
                    item_type: item
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .into(),
                })
            })
            .collect());
    }
    if state.local_api {
        let encoded: String = query
            .bytes()
            .map(|byte| match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    (byte as char).to_string()
                }
                _ => format!("%{byte:02X}"),
            })
            .collect();
        let (status, text) = get_text(&format!(
            "/api/users/0/items/top?q={encoded}&qmode=titleCreatorYear&limit=30&format=json"
        ))?;
        if status != 200 {
            return Err(format!("Zotero antwortete mit HTTP {status}."));
        }
        let items: Vec<Value> = serde_json::from_str(&text)
            .map_err(|_| "Zotero lieferte keine gültige Antwort.".to_string())?;
        return Ok(items
            .iter()
            .filter_map(|item| {
                let data = item.get("data")?;
                let item_type = data.get("itemType").and_then(Value::as_str).unwrap_or("");
                if matches!(item_type, "attachment" | "note" | "annotation") {
                    return None;
                }
                Some(ZoteroItem {
                    id: item.get("key").and_then(Value::as_str)?.to_string(),
                    source: "local".into(),
                    title: data
                        .get("title")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .into(),
                    authors: creators_text(
                        data.get("creators")
                            .and_then(Value::as_array)
                            .map(Vec::as_slice)
                            .unwrap_or(&[]),
                    ),
                    year: year_of(data.get("date").and_then(Value::as_str).unwrap_or("")),
                    item_type: item_type.into(),
                })
            })
            .collect());
    }
    Err(state.message)
}

/// BibTeX-Einträge der gewählten Elemente.
pub fn bibtex(ids: &[String], source: &str) -> Result<String, String> {
    if ids.is_empty() {
        return Ok(String::new());
    }
    match source {
        "bbt" => {
            let result = rpc("item.export", json!([ids, "Better BibTeX"]))?;
            match result {
                Value::String(text) => Ok(text),
                // Ältere BBT-Versionen: [status, format, text]
                Value::Array(parts) => parts
                    .iter()
                    .rev()
                    .find_map(|part| part.as_str().map(str::to_string))
                    .ok_or_else(|| "Better BibTeX lieferte keinen BibTeX-Text.".into()),
                _ => Err("Better BibTeX lieferte keinen BibTeX-Text.".into()),
            }
        }
        _ => {
            if ids
                .iter()
                .any(|id| !id.chars().all(|c| c.is_ascii_alphanumeric()))
            {
                return Err("Ungültiger Zotero-Schlüssel.".into());
            }
            let (status, text) = get_text(&format!(
                "/api/users/0/items?itemKey={}&format=bibtex",
                ids.join(",")
            ))?;
            if status != 200 {
                return Err(format!("Zotero antwortete mit HTTP {status}."));
            }
            Ok(text)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_creators_and_years() {
        let creators = vec![
            json!({ "lastName": "Knuth" }),
            json!({ "family": "Lamport" }),
            json!({ "name": "Team" }),
        ];
        assert_eq!(creators_text(&creators[..1]), "Knuth");
        assert_eq!(creators_text(&creators[..2]), "Knuth und Lamport");
        assert_eq!(creators_text(&creators), "Knuth u. a.");
        assert_eq!(year_of("März 1994"), "1994");
        assert_eq!(year_of(""), "");
    }
}
