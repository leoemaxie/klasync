//! Operations on materials that the HTTP layer and background code share:
//! saving the working draft, publishing, unpublishing and exporting.

use std::fmt;

use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    blocks::{self, MaterialDoc},
    book,
    delivery,
    generate,
    registry::{registry, Audience, RenderContext, RenderError, RenderOptions, Target},
    render::{file_stem, render_with},
    store::{self, MaterialRow, MaterialSettings},
};
use crate::state::AppState;

#[derive(Debug)]
pub enum ServiceError {
    NotFound(&'static str),
    Invalid(String),
    Conflict(&'static str),
    Unavailable(String),
    Internal(String),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(m) | Self::Conflict(m) => write!(f, "{m}"),
            Self::Invalid(m) | Self::Unavailable(m) | Self::Internal(m) => write!(f, "{m}"),
        }
    }
}

impl From<sqlx::Error> for ServiceError {
    fn from(error: sqlx::Error) -> Self {
        Self::Internal(error.to_string())
    }
}

impl From<String> for ServiceError {
    fn from(message: String) -> Self {
        Self::Internal(message)
    }
}

// ---- draft ----------------------------------------------------------------------------

/// Saves the lecturer's edits. `base_rev` must be the revision the editor
/// loaded, so two editors (or an editor and a regeneration) cannot silently
/// overwrite each other.
pub async fn save_draft(
    pool: &PgPool,
    material: &MaterialRow,
    mut doc: MaterialDoc,
    base_rev: i32,
) -> Result<i32, ServiceError> {
    doc.normalise_ids("b");
    doc.number_equations(1);
    blocks::validate(&doc).map_err(|problems| ServiceError::Invalid(problems.join("; ")))?;
    let rev: Option<i32> = sqlx::query_scalar(
        "update lecture_materials set draft_doc = $2, draft_rev = draft_rev + 1, updated_at = now()
         where id = $1 and draft_rev = $3 returning draft_rev",
    )
    .bind(material.id)
    .bind(serde_json::to_value(&doc).map_err(|e| ServiceError::Internal(e.to_string()))?)
    .bind(base_rev)
    .fetch_optional(pool)
    .await?;
    rev.ok_or(ServiceError::Conflict(
        "The draft changed since you opened it. Reload to see the latest version.",
    ))
}

/// draft <-> in_review. Publishing and archiving have their own operations.
pub async fn set_review_status(
    pool: &PgPool,
    material: &MaterialRow,
    status: &str,
) -> Result<(), ServiceError> {
    if !matches!(status, "draft" | "in_review") {
        return Err(ServiceError::Invalid("status must be draft or in_review".into()));
    }
    if material.status == "published" || material.status == "archived" {
        return Err(ServiceError::Conflict("A published or archived material cannot go back to review."));
    }
    sqlx::query("update lecture_materials set status = $2, updated_at = now() where id = $1")
        .bind(material.id)
        .bind(status)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_template(pool: &PgPool, material: &MaterialRow, template: &str) -> Result<(), ServiceError> {
    if !registry().templates().iter().any(|t| t.id == template) {
        return Err(ServiceError::Invalid("unknown template".into()));
    }
    sqlx::query("update lecture_materials set template = $2, updated_at = now() where id = $1")
        .bind(material.id)
        .bind(template)
        .execute(pool)
        .await?;
    Ok(())
}

/// Queues a rewrite of one section of the working draft.
pub async fn request_regeneration(
    pool: &PgPool,
    material: &MaterialRow,
    heading_id: &str,
    instruction: Option<&str>,
    requested_by: Uuid,
) -> Result<(), ServiceError> {
    let draft = material
        .draft_doc
        .as_ref()
        .ok_or_else(|| ServiceError::Invalid("There is no draft yet.".into()))?;
    let doc = store::parse_doc(draft).map_err(ServiceError::Internal)?;
    if !doc.sections().iter().any(|s| s.block_id == heading_id) {
        return Err(ServiceError::NotFound("That section is not in the draft."));
    }
    let instruction = instruction.map(str::trim).filter(|i| !i.is_empty()).map(|i| i.chars().take(500).collect::<String>());
    store::enqueue_job(
        pool,
        material.session_id,
        requested_by,
        "material_section",
        json!({
            "material_id": material.id, "generation": material.generation, "mode": "regenerate",
            "heading_id": heading_id, "instruction": instruction, "nonce": Uuid::now_v7().to_string(),
        }),
    )
    .await?;
    Ok(())
}

/// Starts a full (re)generation. Fails when one is already running.
pub async fn request_generation(
    pool: &PgPool,
    material: &MaterialRow,
    requested_by: Uuid,
) -> Result<i32, ServiceError> {
    generate::start_generation(pool, material, requested_by, 0)
        .await
        .map_err(|e| {
            if e.contains("already running") {
                ServiceError::Conflict("Generation is already running for this lecture.")
            } else {
                ServiceError::Internal(e)
            }
        })
}

// ---- publish ----------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct Published {
    pub version_id: Uuid,
    pub version_no: i32,
    pub resource_id: Option<Uuid>,
    pub deliveries: Vec<Uuid>,
}

#[derive(Debug, Default)]
pub struct PublishOptions {
    /// Delivery channel ids to trigger (`email`, `link`, `in_app`).
    pub deliver: Vec<String>,
    pub message: Option<String>,
}

/// Publishes the working draft as an immutable version.
pub async fn publish(
    state: &AppState,
    material: &MaterialRow,
    lecturer_id: Uuid,
    options: PublishOptions,
) -> Result<Published, ServiceError> {
    let pool = state.db_pool();
    if material.generation_state == "queued" || material.generation_state == "generating" {
        return Err(ServiceError::Conflict("Wait for generation to finish before publishing."));
    }
    for channel in &options.deliver {
        if registry().delivery_channel(channel).is_none() {
            return Err(ServiceError::Invalid(format!("unknown delivery channel {channel}")));
        }
    }
    let draft = material
        .draft_doc
        .as_ref()
        .ok_or_else(|| ServiceError::Invalid("There is nothing to publish yet.".into()))?;
    let mut doc = store::parse_doc(draft).map_err(ServiceError::Internal)?;
    blocks::validate(&doc).map_err(|p| ServiceError::Invalid(p.join("; ")))?;
    // Publishing is the lecturer's sign-off.
    doc.meta.ai_drafted = false;

    let mut tx = pool.begin().await?;
    let locked: Option<i32> = sqlx::query_scalar(
        "select draft_rev from lecture_materials where id = $1 for update",
    )
    .bind(material.id)
    .fetch_optional(&mut *tx)
    .await?;
    if locked != Some(material.draft_rev) {
        return Err(ServiceError::Conflict("The draft changed while publishing. Please try again."));
    }
    let (version_id, version_no) =
        store::insert_version(&mut tx, material.id, "lecturer", &doc, None, None, None, true).await?;
    sqlx::query(
        "update lecture_materials
         set status = 'published', published_version_id = $2, current_version_id = $2,
             draft_doc = $3, draft_rev = draft_rev + 1, updated_at = now()
         where id = $1",
    )
    .bind(material.id)
    .bind(version_id)
    .bind(serde_json::to_value(&doc).map_err(|e| ServiceError::Internal(e.to_string()))?)
    .execute(&mut *tx)
    .await?;
    store::emit(
        &mut *tx,
        "material.published",
        json!({"material_id": material.id, "session_id": material.session_id, "course_id": material.course_id,
               "version_id": version_id, "version_no": version_no, "lecturer_id": lecturer_id}),
    )
    .await?;
    book::mark_stale(&mut tx, material.course_id).await?;
    tx.commit().await?;

    let resource_id = match publish_resource(state, material, version_id).await {
        Ok(id) => id,
        Err(error) => {
            // The material is published either way; students can still read it in the app.
            tracing::warn!(%error, material = %material.id, "Could not store the published export");
            None
        }
    };

    let title = doc.title.clone();
    let mut deliveries = Vec::new();
    for channel in &options.deliver {
        let id = delivery::request(
            pool,
            channel,
            Target::Version(version_id),
            material.course_id,
            &title,
            &Audience::EnrolledRoster,
            options.message.as_deref(),
            lecturer_id,
        )
        .await?;
        deliveries.push(id);
    }
    Ok(Published { version_id, version_no, resource_id, deliveries })
}

/// The format students get by default: PDF when this server can make one.
fn default_resource_format() -> &'static str {
    match registry().renderer("pdf") {
        Some(pdf) if pdf.available() => "pdf",
        _ => "docx",
    }
}

/// Replaces the session's material `notes` resource with the new version's export.
async fn publish_resource(
    state: &AppState,
    material: &MaterialRow,
    version_id: Uuid,
) -> Result<Option<Uuid>, ServiceError> {
    let pool = state.db_pool();
    let exported = export(state, Target::Version(version_id), default_resource_format(), None).await?;
    let stored = state
        .storage
        .put(&exported.filename, exported.bytes.clone())
        .await
        .map_err(|e| ServiceError::Unavailable(e.to_string()))?;
    let id = Uuid::now_v7();
    let mut tx = pool.begin().await?;
    sqlx::query(
        "delete from lecture_resources
         where session_id = $1 and resource_type = 'notes' and material_version_id is not null",
    )
    .bind(material.session_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "insert into lecture_resources
           (id, session_id, resource_type, storage_key, checksum, content_type, original_filename, byte_size, material_version_id)
         values ($1, $2, 'notes', $3, $4, $5, $6, $7, $8)",
    )
    .bind(id)
    .bind(material.session_id)
    .bind(&stored.key)
    .bind(sha256_hex(&exported.bytes))
    .bind(exported.content_type)
    .bind(&exported.filename)
    .bind(exported.bytes.len() as i64)
    .bind(version_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Some(id))
}

/// Hides the material from students. Versions are kept.
pub async fn unpublish(
    pool: &PgPool,
    material: &MaterialRow,
    archive: bool,
) -> Result<(), ServiceError> {
    if material.status != "published" && !(archive && material.status != "archived") {
        return Err(ServiceError::Conflict("This material is not published."));
    }
    let mut tx = pool.begin().await?;
    sqlx::query(
        "update lecture_materials set status = $2, published_version_id = null, updated_at = now() where id = $1",
    )
    .bind(material.id)
    .bind(if archive { "archived" } else { "draft" })
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "delete from lecture_resources
         where session_id = $1 and resource_type = 'notes' and material_version_id is not null",
    )
    .bind(material.session_id)
    .execute(&mut *tx)
    .await?;
    store::emit(
        &mut *tx,
        "material.unpublished",
        json!({"material_id": material.id, "session_id": material.session_id, "course_id": material.course_id}),
    )
    .await?;
    book::mark_stale(&mut tx, material.course_id).await?;
    tx.commit().await?;
    Ok(())
}

// ---- export -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Exported {
    pub bytes: Vec<u8>,
    pub content_type: &'static str,
    pub filename: String,
    pub cached: bool,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn static_content_type(value: &str) -> &'static str {
    match value {
        v if v.starts_with("application/pdf") => "application/pdf",
        v if v.starts_with("application/vnd.openxmlformats") => {
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        }
        v if v.starts_with("text/markdown") => "text/markdown; charset=utf-8",
        v if v.starts_with("text/html") => "text/html; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn render_error(error: RenderError) -> ServiceError {
    match error {
        RenderError::Unavailable(m) => ServiceError::Unavailable(m),
        RenderError::Failed(m) => ServiceError::Internal(m),
    }
}

/// Renders (or fetches from the cache) a version or book in a format.
/// Exports are cached in object storage per version, format and template.
pub async fn export(
    state: &AppState,
    target: Target,
    format: &str,
    template: Option<&str>,
) -> Result<Exported, ServiceError> {
    let pool = state.db_pool();
    let renderer = registry()
        .renderer(format)
        .ok_or_else(|| ServiceError::Invalid(format!("unknown format {format}")))?;
    if !renderer.available() {
        return Err(ServiceError::Unavailable(format!(
            "{} export is not available on this server",
            renderer.label()
        )));
    }

    // Everything the renderer needs, per target kind.
    let (doc, course_id, heading, template_id, version_label, options) = match target {
        Target::Version(version_id) => {
            let version = store::find_version(pool, version_id)
                .await?
                .ok_or(ServiceError::NotFound("Version not found"))?;
            let material = store::find_material(pool, version.material_id)
                .await?
                .ok_or(ServiceError::NotFound("Material not found"))?;
            let session = store::session_info(pool, material.session_id).await?;
            let doc = store::parse_doc(&version.doc).map_err(ServiceError::Internal)?;
            (
                doc,
                material.course_id,
                session.map(|s| s.title).unwrap_or_else(|| "Lecture".to_owned()),
                template.map(str::to_owned).unwrap_or(material.template),
                format!("v{}", version.version_no),
                RenderOptions::default(),
            )
        }
        Target::Book(book_version_id) => {
            let version = book::find_book_version(pool, book_version_id)
                .await?
                .ok_or(ServiceError::NotFound("Book version not found"))?;
            let book_row = sqlx::query_as::<_, (Uuid, String)>(
                "select course_id, title from course_books where id = $1",
            )
            .bind(version.book_id)
            .fetch_one(pool)
            .await?;
            let doc = store::parse_doc(&version.doc).map_err(ServiceError::Internal)?;
            let settings: MaterialSettings = store::load_settings(pool, book_row.0).await;
            (
                doc,
                book_row.0,
                book_row.1,
                template.map(str::to_owned).unwrap_or(settings.template),
                format!("book v{}", version.version_no),
                RenderOptions { toc: true, chapter_breaks: true },
            )
        }
    };
    let template = registry().template(&template_id);
    let course = store::course_info(pool, course_id)
        .await?
        .ok_or(ServiceError::NotFound("Course not found"))?;
    let context = RenderContext {
        course_code: course.code.clone(),
        course_title: course.title.clone(),
        heading,
        lecturer_name: course.lecturer_name.clone(),
        academic_session: Some(course.academic_session.clone()),
        semester: Some(course.semester.clone()),
        generated_at: chrono::Utc::now(),
        version_label: Some(version_label),
    };
    let filename = format!("{}.{}", file_stem(&context), renderer.extension());

    let (version_col, book_col) = match target {
        Target::Version(id) => (Some(id), None),
        Target::Book(id) => (None, Some(id)),
    };
    let cached = sqlx::query_as::<_, (String, String)>(
        "select storage_key, content_type from material_exports
         where version_id is not distinct from $1 and book_version_id is not distinct from $2
           and format = $3 and template = $4",
    )
    .bind(version_col)
    .bind(book_col)
    .bind(format)
    .bind(template.id)
    .fetch_optional(pool)
    .await?;
    if let Some((key, content_type)) = cached {
        if let Ok(bytes) = state.storage.get(&key).await {
            return Ok(Exported {
                bytes,
                content_type: static_content_type(&content_type),
                filename,
                cached: true,
            });
        }
        // The object is gone; fall through and render again.
    }

    let file = render_with(renderer.as_ref(), &doc, &context, template, options)
        .await
        .map_err(render_error)?;
    // Caching is best-effort: a storage hiccup must not fail the download.
    match state.storage.put(&filename, file.bytes.clone()).await {
        Ok(stored) => {
            let _ = sqlx::query(
                "insert into material_exports (version_id, book_version_id, format, template, storage_key, content_type, byte_size, checksum)
                 values ($1, $2, $3, $4, $5, $6, $7, $8)
                 on conflict do nothing",
            )
            .bind(version_col)
            .bind(book_col)
            .bind(format)
            .bind(template.id)
            .bind(&stored.key)
            .bind(file.content_type)
            .bind(file.bytes.len() as i64)
            .bind(sha256_hex(&file.bytes))
            .execute(pool)
            .await;
        }
        Err(error) => tracing::debug!(%error, "Export not cached"),
    }
    Ok(Exported {
        bytes: file.bytes,
        content_type: file.content_type,
        filename,
        cached: false,
    })
}

/// Renders the lecturer's working draft on the fly (never cached), so they
/// can preview or export before publishing.
pub async fn export_draft(
    state: &AppState,
    material: &MaterialRow,
    format: &str,
    template: Option<&str>,
) -> Result<Exported, ServiceError> {
    let pool = state.db_pool();
    let renderer = registry()
        .renderer(format)
        .ok_or_else(|| ServiceError::Invalid(format!("unknown format {format}")))?;
    if !renderer.available() {
        return Err(ServiceError::Unavailable(format!(
            "{} export is not available on this server",
            renderer.label()
        )));
    }
    let draft = material
        .draft_doc
        .as_ref()
        .ok_or(ServiceError::NotFound("There is no draft yet."))?;
    let doc = store::parse_doc(draft).map_err(ServiceError::Internal)?;
    let course = store::course_info(pool, material.course_id)
        .await?
        .ok_or(ServiceError::NotFound("Course not found"))?;
    let session = store::session_info(pool, material.session_id).await?;
    let context = RenderContext {
        course_code: course.code.clone(),
        course_title: course.title.clone(),
        heading: session.map(|s| s.title).unwrap_or_else(|| "Lecture".to_owned()),
        lecturer_name: course.lecturer_name.clone(),
        academic_session: Some(course.academic_session.clone()),
        semester: Some(course.semester.clone()),
        generated_at: chrono::Utc::now(),
        version_label: Some("draft".to_owned()),
    };
    let template = registry().template(template.unwrap_or(&material.template));
    let file = render_with(renderer.as_ref(), &doc, &context, template, RenderOptions::default())
        .await
        .map_err(render_error)?;
    Ok(Exported {
        bytes: file.bytes,
        content_type: file.content_type,
        filename: format!("{}-draft.{}", file_stem(&context), renderer.extension()),
        cached: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_types_map_to_static_values() {
        assert_eq!(static_content_type("application/pdf"), "application/pdf");
        assert!(static_content_type("application/vnd.openxmlformats-officedocument.wordprocessingml.document").contains("wordprocessing"));
        assert_eq!(static_content_type("text/markdown; charset=utf-8"), "text/markdown; charset=utf-8");
        assert_eq!(static_content_type("weird"), "application/octet-stream");
    }

    #[test]
    fn sha256_is_hex() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn default_format_is_pdf_or_docx() {
        assert!(matches!(default_resource_format(), "pdf" | "docx"));
    }
}
