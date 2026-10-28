use std::{sync::LazyLock, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use hc_helper::airtable::{self as hc_airtable, AirtableTable};
use hc_helper::submission::UNIFIED_FIELDS;
use reqwest::{Client, Url};
use serde_json::{json, Value};
use tracing::{error, info, warn};

pub const SCREENSHOT_FIELD: &str = "Screenshot";
pub const PRIZE_TIERS: [&str; 3] = ["Thumby Color", "LILYGO T-Deck Max", "Both"];

static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(60))
        .build()
        .expect("failed to build HTTP client")
});

pub fn table() -> AirtableTable {
    AirtableTable {
        token: hc_helper::keys::airtable_token(),
        base_id: std::env::var("AIRTABLE_BASE_ID").unwrap_or_default(),
        table_id_or_name: std::env::var("AIRTABLE_TABLE").unwrap_or_default(),
    }
}

fn expected_fields() -> Vec<Value> {
    let text = |name: &str| json!({ "name": name, "type": "singleLineText" });
    vec![
        json!({ "name": "Code URL", "type": "url" }),
        json!({ "name": "Playable URL", "type": "url" }),
        text("First Name"),
        text("Last Name"),
        json!({ "name": "Email", "type": "email" }),
        json!({ "name": SCREENSHOT_FIELD, "type": "multipleAttachments" }),
        json!({ "name": "Description", "type": "multilineText" }),
        text("Address (Line 1)"),
        text("Address (Line 2)"),
        text("City"),
        text("State / Province"),
        text("ZIP / Postal Code"),
        text("Country"),
        json!({ "name": "Birthday", "type": "date", "options": { "dateFormat": { "name": "iso" } } }),
        json!({
            "name": "Justification - Hackatime Project Name(s) + Date Range(s)",
            "type": "multilineText"
        }),
        json!({
            "name": "Prize Tier",
            "type": "singleSelect",
            "options": { "choices": PRIZE_TIERS.map(|name| json!({ "name": name })) }
        }),
        text("Slack ID"),
        text("HCA ID"),
        json!({ "name": "Hackatime Projects", "type": "multilineText" }),
        json!({ "name": "Phone Number", "type": "phoneNumber" }),
    ]
}

pub async fn ensure_schema() {
    match try_ensure_schema().await {
        Ok(()) => info!("Airtable schema ok"),
        Err(e) => error!(
            error = %format!("{e:#}"),
            "AIRTABLE IS NOT SET UP CORRECTLY, SUBMISSIONS WILL FAIL"
        ),
    }
}

async fn try_ensure_schema() -> Result<()> {
    let table = table();
    if table.token.is_empty() || table.base_id.is_empty() || table.table_id_or_name.is_empty() {
        bail!("AIRTABLE_TOKEN, AIRTABLE_BASE_ID and AIRTABLE_TABLE must all be set");
    }

    let expected = expected_fields();
    for name in UNIFIED_FIELDS {
        if !expected.iter().any(|f| f["name"] == *name) {
            bail!("hc_helper sends a field chipper doesn't know about: {name}");
        }
    }

    let tables = hc_airtable::tables(&table.token, &table.base_id)
        .await
        .context("couldn't read the base schema")?;
    let Some(existing) = tables
        .into_iter()
        .find(|t| t.id == table.table_id_or_name || t.name == table.table_id_or_name)
    else {
        warn!(table = %table.table_id_or_name, "Airtable table missing, creating it");
        hc_airtable::create_table(
            &table.token,
            &table.base_id,
            &table.table_id_or_name,
            &expected,
        )
        .await
        .context("couldn't create the table")?;
        return Ok(());
    };

    let mut problems = 0;
    for field in &expected {
        let name = field["name"].as_str().unwrap_or_default();
        let kind = field["type"].as_str().unwrap_or_default();
        match existing.fields.iter().find(|f| f.name == name) {
            Some(f) if f.kind == kind || (kind == "multilineText" && f.kind == "richText") => {}
            Some(f) => {
                error!(field = name, expected = kind, actual = %f.kind, "Airtable field has the wrong type");
                problems += 1;
            }
            None => {
                warn!(field = name, "Airtable field missing, creating it");
                if let Err(e) =
                    hc_airtable::create_field(&table.token, &table.base_id, &existing.id, field)
                        .await
                {
                    error!(field = name, error = %format!("{e:#}"), "couldn't create Airtable field");
                    problems += 1;
                }
            }
        }
    }

    if problems > 0 {
        bail!("{problems} field problem(s), see the errors above");
    }
    Ok(())
}

pub async fn upload_screenshot(
    record_id: &str,
    content_type: &str,
    filename: &str,
    bytes: &[u8],
) -> Result<()> {
    let table = table();
    let mut url = Url::parse("https://content.airtable.com")?;
    url.path_segments_mut()
        .map_err(|()| anyhow!("bad Airtable content URL"))?
        .extend([
            "v0",
            &table.base_id,
            record_id,
            SCREENSHOT_FIELD,
            "uploadAttachment",
        ]);

    let response = CLIENT
        .post(url)
        .bearer_auth(&table.token)
        .json(&json!({
            "contentType": content_type,
            "file": STANDARD.encode(bytes),
            "filename": filename,
        }))
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("Airtable responded {status}: {body}");
    }
    Ok(())
}
