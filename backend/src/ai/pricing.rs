//! Price table used to turn normalised token counts into dollars.
//!
//! Gemini does not report a cost with its responses, so the application
//! computes it. Prices change on known dates (see the scheduled entries), so
//! every entry carries an `effective_from` date and the newest applicable one
//! wins. Operators can override or extend the table with `AI_PRICE_OVERRIDES`
//! (JSON) without a deploy.

use std::collections::HashMap;

use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;

use super::types::Usage;

/// USD per 1M tokens.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub struct Price {
    pub input: f64,
    pub output: f64,
    /// Audio input, when priced separately from text.
    #[serde(default)]
    pub audio_input: Option<f64>,
    #[serde(default)]
    pub cached_input: Option<f64>,
}

impl Price {
    const fn new(input: f64, output: f64) -> Self {
        Self {
            input,
            output,
            audio_input: None,
            cached_input: None,
        }
    }
    const fn with_audio(mut self, audio: f64) -> Self {
        self.audio_input = Some(audio);
        self
    }
    const fn with_cache(mut self, cached: f64) -> Self {
        self.cached_input = Some(cached);
        self
    }
}

#[derive(Debug, Clone)]
struct Entry {
    model: String,
    effective_from: DateTime<Utc>,
    price: Price,
}

#[derive(Debug, Clone)]
pub struct PriceTable {
    entries: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
struct OverrideEntry {
    model: String,
    #[serde(default)]
    effective_from: Option<DateTime<Utc>>,
    #[serde(flatten)]
    price: Price,
}

fn day(y: i32, m: u32, d: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, 0, 0, 0)
        .single()
        .expect("valid date")
}

impl PriceTable {
    /// Rates from Google's published Gemini API pricing (paid tier, standard),
    /// as listed in AI Studio on 2026-11-25.
    pub fn defaults() -> Self {
        let epoch = day(2020, 1, 1);
        let mut entries = Vec::new();
        let mut add = |model: &str, from: DateTime<Utc>, price: Price| {
            entries.push(Entry {
                model: model.to_owned(),
                effective_from: from,
                price,
            });
        };
        add(
            "gemini-3.5-flash-lite",
            epoch,
            Price::new(0.30, 2.50).with_cache(0.03),
        );
        add(
            "gemini-3.1-flash-lite",
            epoch,
            Price::new(0.25, 1.50).with_audio(0.50).with_cache(0.025),
        );
        add(
            "gemini-2.5-flash-lite",
            epoch,
            Price::new(0.10, 0.40).with_audio(0.30),
        );
        add("gemini-2.5-flash", epoch, Price::new(0.30, 2.50));
        // Introductory pricing that doubles on 1 Jan 2027.
        for model in [
            "gemini-3.8-flash",
            "gemini-3.7-flash",
            "gemini-3.6-flash",
            "gemini-3.5-flash",
        ] {
            add(model, epoch, Price::new(0.75, 3.75).with_cache(0.075));
            add(
                model,
                day(2027, 1, 1),
                Price::new(1.50, 7.50).with_cache(0.15),
            );
        }
        add(
            "gemini-3-flash-preview",
            epoch,
            Price::new(0.50, 3.00).with_audio(1.00),
        );
        add(
            "gemini-4-hydrogen",
            epoch,
            Price::new(1.50, 7.50).with_cache(0.075),
        );
        add(
            "gemini-4-neon",
            epoch,
            Price::new(3.00, 15.00).with_cache(0.15),
        );
        // Introductory pricing that doubles on 1 Dec 2026.
        for model in ["gemini-4-carbon", "gemini-4-argon"] {
            add(model, epoch, Price::new(2.00, 10.00).with_cache(0.10));
            add(
                model,
                day(2026, 12, 1),
                Price::new(4.00, 20.00).with_cache(0.20),
            );
        }
        add("gemini-3.1-pro-preview", epoch, Price::new(2.00, 12.00));
        add("gemini-2.5-pro", epoch, Price::new(1.25, 10.00));
        // Speech to text: $2/M audio tokens in, $12/M text out.
        add(
            "gemini-3.5-transcribe",
            epoch,
            Price::new(2.00, 12.00).with_audio(2.00),
        );
        add("gemini-embedding-2", epoch, Price::new(0.20, 0.0));
        add("gemini-embedding-001", epoch, Price::new(0.15, 0.0));
        Self { entries }
    }

    /// Applies `AI_PRICE_OVERRIDES` (a JSON array of
    /// `{"model","input","output",...,"effective_from"?}`); bad JSON is ignored
    /// with a warning rather than taking the service down.
    pub fn with_overrides(mut self, raw: Option<&str>) -> Self {
        if let Some(raw) = raw.filter(|raw| !raw.trim().is_empty()) {
            match serde_json::from_str::<Vec<OverrideEntry>>(raw) {
                Ok(items) => {
                    for item in items {
                        self.entries.push(Entry {
                            model: item.model,
                            effective_from: item.effective_from.unwrap_or_else(|| day(2020, 1, 2)),
                            price: item.price,
                        });
                    }
                }
                Err(error) => tracing::warn!(%error, "Ignoring invalid AI_PRICE_OVERRIDES"),
            }
        }
        self
    }

    pub fn price_for(&self, model: &str, at: DateTime<Utc>) -> Option<Price> {
        let model = model.trim_start_matches("models/");
        self.entries
            .iter()
            .filter(|entry| entry.model == model && entry.effective_from <= at)
            .max_by_key(|entry| entry.effective_from)
            .map(|entry| entry.price)
    }

    /// Dollar cost of one call. A provider-reported cost always wins; an
    /// unknown model costs zero but is logged so the gap is visible.
    pub fn cost(&self, model: &str, usage: &Usage, at: DateTime<Utc>) -> f64 {
        if let Some(cost) = usage.reported_cost_usd {
            return cost;
        }
        let Some(price) = self.price_for(model, at) else {
            if !model.contains("free") {
                tracing::debug!(%model, "No price configured for model; counting it as free");
            }
            return 0.0;
        };
        let audio = usage.audio_input_tokens.min(usage.input_tokens);
        let cached = usage.cached_tokens.min(usage.input_tokens - audio);
        let text_in = usage.input_tokens - audio - cached;
        let per_million = 1_000_000.0;
        text_in as f64 * price.input / per_million
            + cached as f64 * price.cached_input.unwrap_or(price.input) / per_million
            + audio as f64 * price.audio_input.unwrap_or(price.input) / per_million
            + usage.output_tokens as f64 * price.output / per_million
    }

    pub fn known_models(&self) -> HashMap<&str, usize> {
        let mut counts = HashMap::new();
        for entry in &self.entries {
            *counts.entry(entry.model.as_str()).or_insert(0) += 1;
        }
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduled_price_changes_apply_on_their_date() {
        let table = PriceTable::defaults();
        let before = table
            .price_for("gemini-4-carbon", day(2026, 11, 30))
            .unwrap();
        let after = table
            .price_for("gemini-4-carbon", day(2026, 12, 1))
            .unwrap();
        assert_eq!((before.input, before.output), (2.0, 10.0));
        assert_eq!((after.input, after.output), (4.0, 20.0));
        let flash_now = table
            .price_for("gemini-3.8-flash", day(2026, 11, 25))
            .unwrap();
        let flash_2027 = table
            .price_for("gemini-3.8-flash", day(2027, 1, 2))
            .unwrap();
        assert_eq!(flash_now.input, 0.75);
        assert_eq!(flash_2027.input, 1.50);
    }

    #[test]
    fn cost_counts_thinking_as_output_and_prices_audio_separately() {
        let table = PriceTable::defaults();
        let usage = Usage {
            input_tokens: 1_000_000,
            output_tokens: 1_000_000,
            ..Usage::default()
        };
        let cost = table.cost("gemini-3.5-flash-lite", &usage, day(2026, 11, 25));
        assert!((cost - 2.80).abs() < 1e-9);

        let audio = Usage {
            input_tokens: 1_500_000,
            audio_input_tokens: 1_500_000,
            ..Usage::default()
        };
        let cost = table.cost("gemini-3.5-transcribe", &audio, day(2026, 11, 25));
        assert!((cost - 3.0).abs() < 1e-9);
    }

    #[test]
    fn reported_cost_and_overrides_win() {
        let table = PriceTable::defaults().with_overrides(Some(
            r#"[{"model":"gemini-4-hydrogen","input":1.0,"output":2.0,"effective_from":"2026-11-01T00:00:00Z"}]"#,
        ));
        let usage = Usage {
            input_tokens: 1_000_000,
            ..Usage::default()
        };
        assert_eq!(
            table.cost("gemini-4-hydrogen", &usage, day(2026, 11, 25)),
            1.0
        );
        let reported = Usage {
            reported_cost_usd: Some(0.5),
            ..usage
        };
        assert_eq!(table.cost("anything", &reported, day(2026, 11, 25)), 0.5);
    }
}
