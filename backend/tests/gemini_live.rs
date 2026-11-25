//! Live smoke tests against the real Gemini API. Ignored by default (they cost
//! a fraction of a cent and need a key):
//!
//!     GEMINI_API_KEY=... cargo test --test gemini_live -- --ignored --nocapture
//!
//! Optional: GEMINI_TEST_WAV=/path/to/speech.wav for the transcription test.

use klasync_backend::{
    ai::{
        gateway_from_config, AiGateway, EmbedItem, EmbedKind, Task, TextRequest, Thinking,
        TranscribeRequest,
    },
    config::AppConfig,
};

fn gateway() -> std::sync::Arc<AiGateway> {
    let config = AppConfig::from_env();
    assert!(
        config.gemini_api_key.is_some(),
        "set GEMINI_API_KEY to run the live tests"
    );
    gateway_from_config(&config, None)
}

#[tokio::test]
#[ignore]
async fn text_generation_returns_json_on_the_cheap_and_mid_tiers() {
    let gw = gateway();
    for task in [Task::RollingSummary, Task::Flashcards] {
        let request = TextRequest::new(
            task,
            "Return only JSON: an array of two objects with keys prompt and answer.",
            "Photosynthesis converts light energy into chemical energy in chloroplasts.",
        )
        .json()
        .thinking(Thinking::Minimal)
        .max_tokens(600);
        let out = gw.generate(&request).await.expect("generation");
        let json = klasync_backend::ai::extract_json(&out.text).expect("valid JSON");
        assert!(json.is_array(), "{task:?}: {}", out.text);
        let cost = gw.prices().cost(&out.model, &out.usage, chrono::Utc::now());
        println!(
            "{task:?}: model={} in={} out={} cost=${cost:.6}",
            out.model, out.usage.input_tokens, out.usage.output_tokens
        );
        assert!(out.usage.input_tokens > 0 && out.usage.output_tokens > 0);
    }
}

#[tokio::test]
#[ignore]
async fn embeddings_have_the_configured_size_and_rank_related_text_higher() {
    let gw = gateway();
    let docs = vec![
        EmbedItem {
            text: "Chlorophyll absorbs red and blue light in the chloroplast.".into(),
            title: Some("Biology".into()),
        },
        EmbedItem {
            text: "The treaty of Westphalia ended the Thirty Years War in 1648.".into(),
            title: Some("History".into()),
        },
    ];
    let docs = gw
        .embed(&docs, EmbedKind::Document, None)
        .await
        .expect("documents");
    assert_eq!(docs.vectors[0].len(), 1536);
    let query = gw
        .embed(
            &[EmbedItem {
                text: "why are leaves green".into(),
                title: None,
            }],
            EmbedKind::Query,
            None,
        )
        .await
        .expect("query");
    let dot = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>();
    let (bio, hist) = (
        dot(&query.vectors[0], &docs.vectors[0]),
        dot(&query.vectors[0], &docs.vectors[1]),
    );
    println!("related={bio:.3} unrelated={hist:.3}");
    assert!(bio > hist + 0.05);
}

#[tokio::test]
#[ignore]
async fn transcription_accepts_wav_and_inlines_or_uploads_by_size() {
    let path = std::env::var("GEMINI_TEST_WAV").unwrap_or_else(|_| "/tmp/lecture.wav".into());
    let audio = std::fs::read(&path).unwrap_or_else(|_| panic!("no test audio at {path}"));
    let gw = gateway();
    let out = gw
        .transcribe(&TranscribeRequest {
            audio,
            mime: "audio/wav".into(),
            language: Some("English".into()),
            duration_sec: Some(16),
            timestamps: true,
            session_id: None,
            job_id: None,
        })
        .await
        .expect("transcription");
    println!("{} -> {:?}", out.model, out.text);
    assert!(out.text.to_lowercase().contains("photosynthesis"));
    assert!(
        out.usage.audio_input_tokens > 0,
        "audio tokens should be reported"
    );
}
