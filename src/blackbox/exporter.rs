use std::fs::File;
use std::io::Write;
use std::path::Path;
use super::error::Result;
use super::sanitizer::sanitize_json_val;
use super::storage::StorageAdapter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Csv,
}

pub struct Exporter;

impl Exporter {
    pub fn export_to_file<P: AsRef<Path>>(
        storage: &StorageAdapter,
        dest_path: P,
        format: ExportFormat,
        mask_pii: bool,
    ) -> Result<()> {
        let mut events = storage.query_all_events()?;
        if mask_pii {
            for ev in &mut events {
                if let Some(ref mut ctx) = ev.context {
                    *ctx = sanitize_json_val(ctx);
                }
                ev.pii_hash = None; // Hide raw pii hash on export
            }
        }

        let mut file = File::create(dest_path)?;

        match format {
            ExportFormat::Json => {
                let json_bytes = serde_json::to_vec_pretty(&events)?;
                file.write_all(&json_bytes)?;
            }
            ExportFormat::Csv => {
                let header = "timestamp_ms,session_id,sequence,event_type,component,action,result,error_code,duration_ms,context_json\n";
                file.write_all(header.as_bytes())?;

                for ev in events {
                    let ctx_str = ev
                        .context
                        .as_ref()
                        .map(|v| v.to_string().replace('"', "\"\""))
                        .unwrap_or_default();

                    let line = format!(
                        "{},{},{},{},{},{},{},{},{},\"{}\"\n",
                        ev.timestamp_ms,
                        ev.session_id,
                        ev.sequence,
                        ev.event_type.as_str(),
                        ev.component,
                        ev.action,
                        ev.result.as_str(),
                        ev.error_code.unwrap_or_default(),
                        ev.duration_ms.unwrap_or(0),
                        ctx_str
                    );
                    file.write_all(line.as_bytes())?;
                }
            }
        }

        file.flush()?;
        Ok(())
    }
}
