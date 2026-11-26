//! Course books: the published materials of one course offering composed into
//! a textbook-style document.
//!
//! Assembly is a pure function of the chapters, structure and front matter
//! (so it is unit tested without a database). It
//!
//! * adds front matter (preface, syllabus, objectives) and chapter headings,
//! * removes repetition: a term defined again in a later lecture becomes a
//!   short "defined in Chapter N" reminder,
//! * keeps equation numbering continuous across chapters,
//! * moves review questions and further reading to the back matter, and
//! * builds a combined glossary and an index from each lecture's key terms.
//!
//! Book versions pin the exact material versions they contain, so a PDF
//! exported in week 5 stays reproducible. Publishing or unpublishing a
//! material marks the book stale; a rebuild reuses cached chapter fragments
//! for unchanged material versions.

use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, PgConnection, PgPool};
use uuid::Uuid;

use super::{
    blocks::{
        self, Block, BlockKind, CalloutVariant, MaterialDoc, ReadingItem, ReviewQuestion,
    },
    store,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct FrontMatter {
    pub subtitle: Option<String>,
    pub preface: Option<String>,
    pub syllabus: Option<String>,
    pub objectives: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UnitSpec {
    pub title: Option<String>,
    #[serde(default)]
    pub material_ids: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub struct ChapterInput {
    pub material_id: Uuid,
    pub version_id: Uuid,
    pub title: String,
    /// Cacheable fragment of the material's blocks (see [`fragment`]).
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone)]
pub struct UnitInput {
    pub title: Option<String>,
    pub chapters: Vec<ChapterInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pin {
    pub material_id: Uuid,
    pub version_id: Uuid,
}

fn short(id: Uuid) -> String {
    id.simple().to_string().chars().take(8).collect()
}

/// A material's blocks prepared for reuse in a book: ids are namespaced by the
/// material so chapters never collide, and equation numbers are cleared (the
/// book numbers them continuously). Independent of the book's structure, so it
/// is safe to cache per material version.
pub fn fragment(doc: &MaterialDoc, material_id: Uuid) -> Vec<Block> {
    let prefix = short(material_id);
    doc.blocks
        .iter()
        .map(|block| {
            let mut block = block.clone();
            block.id = format!("c{prefix}-{}", block.id);
            if let BlockKind::Equation { number, .. } = &mut block.kind {
                *number = None;
            }
            block
        })
        .collect()
}

fn is_key_terms(block: &Block) -> bool {
    matches!(&block.kind, BlockKind::Table { caption: Some(c), .. } if c == "Key terms")
}

/// Pure book assembly. See the module docs.
pub fn assemble(
    title: &str,
    front: &FrontMatter,
    units: &[UnitInput],
    language: &str,
) -> (MaterialDoc, Vec<Pin>) {
    let mut doc = MaterialDoc::new(title);
    doc.subtitle = front.subtitle.clone();
    doc.meta.objectives = front.objectives.clone();
    doc.meta.language = Some(language.to_owned());
    let mut pins: Vec<Pin> = Vec::new();

    let push = |doc: &mut MaterialDoc, id: &str, kind: BlockKind| {
        doc.blocks.push(Block::new(id.to_owned(), kind));
    };
    if let Some(preface) = front.preface.as_deref().filter(|t| !t.trim().is_empty()) {
        push(&mut doc, "front-preface-h", BlockKind::Heading { level: 1, text: "Preface".into() });
        for (i, paragraph) in preface.split("\n\n").filter(|p| !p.trim().is_empty()).enumerate() {
            push(&mut doc, &format!("front-preface-{}", i + 1), BlockKind::Paragraph { text: paragraph.trim().to_owned() });
        }
    }
    if let Some(syllabus) = front.syllabus.as_deref().filter(|t| !t.trim().is_empty()) {
        push(&mut doc, "front-syllabus-h", BlockKind::Heading { level: 1, text: "Syllabus".into() });
        for (i, paragraph) in syllabus.split("\n\n").filter(|p| !p.trim().is_empty()).enumerate() {
            push(&mut doc, &format!("front-syllabus-{}", i + 1), BlockKind::Paragraph { text: paragraph.trim().to_owned() });
        }
    }

    let titled_units = units.iter().any(|u| u.title.is_some());
    let chapter_level: u8 = if titled_units { 2 } else { 1 };

    let mut defined: HashMap<String, usize> = HashMap::new();
    let mut glossary: Vec<(String, String, usize)> = Vec::new();
    let mut index: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut questions: Vec<ReviewQuestion> = Vec::new();
    let mut reading: Vec<ReadingItem> = Vec::new();
    let mut chapter_no = 0usize;

    for (unit_index, unit) in units.iter().enumerate() {
        if let Some(unit_title) = &unit.title {
            push(
                &mut doc,
                &format!("unit-{}-h", unit_index + 1),
                BlockKind::Heading { level: 1, text: unit_title.clone() },
            );
        }
        for chapter in &unit.chapters {
            chapter_no += 1;
            pins.push(Pin { material_id: chapter.material_id, version_id: chapter.version_id });
            let prefix = short(chapter.material_id);
            doc.blocks.push(Block::new(
                format!("c{prefix}-chapter"),
                BlockKind::Heading {
                    level: chapter_level,
                    text: format!("Chapter {chapter_no}: {}", chapter.title),
                },
            ));
            let mut terms_here: Vec<String> = Vec::new();
            for block in &chapter.blocks {
                let mut block = block.clone();
                let key_terms_table = is_key_terms(&block);
                match &mut block.kind {
                    BlockKind::Heading { level, .. } => {
                        *level = (chapter_level + *level).min(4);
                    }
                    BlockKind::ReviewQuestions { questions: qs } => {
                        for q in qs.iter() {
                            let mut q = q.clone();
                            q.question = format!("[Chapter {chapter_no}] {}", q.question);
                            questions.push(q);
                        }
                        continue;
                    }
                    BlockKind::FurtherReading { items } => {
                        for item in items.iter() {
                            if !reading.iter().any(|r| r.title.eq_ignore_ascii_case(&item.title)) {
                                reading.push(item.clone());
                            }
                        }
                        continue;
                    }
                    BlockKind::Definition { term, text } => {
                        let key = blocks::strip_inline(term).to_lowercase();
                        terms_here.push(blocks::strip_inline(term));
                        if let Some(first) = defined.get(&key) {
                            block.kind = BlockKind::Callout {
                                variant: CalloutVariant::Note,
                                text: format!("Recall **{}**, defined in Chapter {first}.", blocks::strip_inline(term)),
                            };
                        } else {
                            defined.insert(key, chapter_no);
                            glossary.push((blocks::strip_inline(term), text.clone(), chapter_no));
                        }
                    }
                    BlockKind::Table { rows, .. } if key_terms_table => {
                        for row in rows.iter() {
                            if let Some(term) = row.first() {
                                terms_here.push(term.clone());
                            }
                        }
                        // Folded into the book index.
                        for term in terms_here.drain(..) {
                            let chapters = index.entry(term).or_default();
                            if !chapters.contains(&chapter_no) {
                                chapters.push(chapter_no);
                            }
                        }
                        continue;
                    }
                    _ => {}
                }
                doc.blocks.push(block);
            }
            for term in terms_here.drain(..) {
                let chapters = index.entry(term).or_default();
                if !chapters.contains(&chapter_no) {
                    chapters.push(chapter_no);
                }
            }
        }
    }

    // Back matter.
    if !questions.is_empty() {
        push(&mut doc, "back-questions-h", BlockKind::Heading { level: 1, text: "Review questions".into() });
        push(&mut doc, "back-questions", BlockKind::ReviewQuestions { questions });
    }
    if !reading.is_empty() {
        push(&mut doc, "back-reading-h", BlockKind::Heading { level: 1, text: "Further reading".into() });
        push(&mut doc, "back-reading", BlockKind::FurtherReading { items: reading });
    }
    if !glossary.is_empty() {
        glossary.sort_by_key(|g| g.0.to_lowercase());
        push(&mut doc, "back-glossary-h", BlockKind::Heading { level: 1, text: "Glossary".into() });
        push(
            &mut doc,
            "back-glossary",
            BlockKind::Table {
                caption: None,
                header: vec!["Term".into(), "Definition".into(), "Chapter".into()],
                rows: glossary
                    .into_iter()
                    .map(|(term, text, chapter)| vec![term, text, chapter.to_string()])
                    .collect(),
            },
        );
    }
    if !index.is_empty() {
        let mut entries: Vec<(String, Vec<usize>)> = index.into_iter().collect();
        entries.sort_by_key(|e| e.0.to_lowercase());
        push(&mut doc, "back-index-h", BlockKind::Heading { level: 1, text: "Index".into() });
        push(
            &mut doc,
            "back-index",
            BlockKind::Table {
                caption: None,
                header: vec!["Term".into(), "Chapters".into()],
                rows: entries
                    .into_iter()
                    .map(|(term, mut chapters)| {
                        chapters.sort_unstable();
                        vec![
                            term,
                            chapters.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", "),
                        ]
                    })
                    .collect(),
            },
        );
    }
    doc.number_equations(1);
    (doc, pins)
}

// ---- persistence ----------------------------------------------------------------------

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct BookRow {
    pub id: Uuid,
    pub course_id: Uuid,
    pub title: String,
    pub front_matter: Value,
    pub structure: Value,
    pub stale: bool,
    #[serde(skip)]
    pub chapter_cache: Value,
    pub published_version_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const BOOK_COLUMNS: &str = "id, course_id, title, front_matter, structure, stale, chapter_cache, published_version_id, created_at, updated_at";

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct BookVersionRow {
    pub id: Uuid,
    pub book_id: Uuid,
    pub version_no: i32,
    pub doc: Value,
    pub pins: Value,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

const BOOK_VERSION_COLUMNS: &str = "id, book_id, version_no, doc, pins, published_at, created_at";

pub async fn find_book(pool: &PgPool, course_id: Uuid) -> Result<Option<BookRow>, sqlx::Error> {
    sqlx::query_as::<_, BookRow>(&format!("select {BOOK_COLUMNS} from course_books where course_id = $1"))
        .bind(course_id)
        .fetch_optional(pool)
        .await
}

pub async fn ensure_book(pool: &PgPool, course_id: Uuid) -> Result<BookRow, String> {
    let course = store::course_info(pool, course_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "course not found".to_owned())?;
    sqlx::query(
        "insert into course_books (course_id, title) values ($1, $2) on conflict (course_id) do nothing",
    )
    .bind(course_id)
    .bind(format!("{} {}: Course book", course.code, course.title))
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    find_book(pool, course_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "the book could not be created".to_owned())
}

pub async fn find_book_version(pool: &PgPool, id: Uuid) -> Result<Option<BookVersionRow>, sqlx::Error> {
    sqlx::query_as::<_, BookVersionRow>(&format!(
        "select {BOOK_VERSION_COLUMNS} from book_versions where id = $1"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn list_book_versions(pool: &PgPool, book_id: Uuid) -> Result<Vec<BookVersionRow>, sqlx::Error> {
    sqlx::query_as::<_, BookVersionRow>(&format!(
        "select {BOOK_VERSION_COLUMNS} from book_versions where book_id = $1 order by version_no desc"
    ))
    .bind(book_id)
    .fetch_all(pool)
    .await
}

/// Marks the course's book stale (and emits `book.stale` on the transition).
pub async fn mark_stale(conn: &mut PgConnection, course_id: Uuid) -> Result<(), sqlx::Error> {
    let changed: Option<Uuid> = sqlx::query_scalar(
        "update course_books set stale = true, updated_at = now()
         where course_id = $1 and not stale returning id",
    )
    .bind(course_id)
    .fetch_optional(&mut *conn)
    .await?;
    if let Some(book_id) = changed {
        store::emit(&mut *conn, "book.stale", json!({"book_id": book_id, "course_id": course_id})).await?;
    }
    Ok(())
}

pub async fn update_book(
    pool: &PgPool,
    course_id: Uuid,
    title: Option<&str>,
    front: Option<&FrontMatter>,
    structure: Option<&[UnitSpec]>,
) -> Result<BookRow, String> {
    let book = ensure_book(pool, course_id).await?;
    sqlx::query(
        "update course_books set
           title = coalesce($2, title),
           front_matter = coalesce($3, front_matter),
           structure = coalesce($4, structure),
           stale = true, updated_at = now()
         where id = $1",
    )
    .bind(book.id)
    .bind(title.map(str::trim).filter(|t| !t.is_empty()))
    .bind(front.map(|f| serde_json::to_value(f).unwrap_or_default()))
    .bind(structure.map(|s| serde_json::to_value(s).unwrap_or_default()))
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    find_book(pool, course_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "the book disappeared".to_owned())
}

#[derive(Debug, FromRow)]
struct PublishedMaterial {
    material_id: Uuid,
    version_id: Uuid,
    lecture_title: String,
    doc: Value,
}

/// Rebuilds the book from the course's published materials and stores it as a
/// new (unpublished) book version. Returns `(version id, version no, chapters
/// rebuilt, chapters reused from cache)`.
pub async fn rebuild(pool: &PgPool, course_id: Uuid) -> Result<(Uuid, i32, usize, usize), String> {
    let book = ensure_book(pool, course_id).await?;
    let course = store::course_info(pool, course_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "course not found".to_owned())?;
    let settings = store::load_settings(pool, course_id).await;
    let materials = sqlx::query_as::<_, PublishedMaterial>(
        "select m.id as material_id, v.id as version_id, s.title as lecture_title, v.doc
         from lecture_materials m
         join lecture_sessions s on s.id = m.session_id
         join material_versions v on v.id = m.published_version_id
         where m.course_id = $1 and m.status = 'published'
         order by s.started_at nulls last, s.title",
    )
    .bind(course_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    if materials.is_empty() {
        return Err("publish at least one lecture material before building the book".to_owned());
    }

    let mut cache: serde_json::Map<String, Value> = book.chapter_cache.as_object().cloned().unwrap_or_default();
    let (mut rebuilt, mut reused) = (0usize, 0usize);
    let mut by_id: HashMap<Uuid, ChapterInput> = HashMap::new();
    let mut order: Vec<Uuid> = Vec::new();
    let mut fresh_cache = serde_json::Map::new();
    for m in &materials {
        let key = m.version_id.to_string();
        let blocks: Vec<Block> = match cache.remove(&key).and_then(|v| serde_json::from_value(v).ok()) {
            Some(blocks) => {
                reused += 1;
                blocks
            }
            None => {
                rebuilt += 1;
                let doc = store::parse_doc(&m.doc)?;
                fragment(&doc, m.material_id)
            }
        };
        fresh_cache.insert(key, serde_json::to_value(&blocks).map_err(|e| e.to_string())?);
        by_id.insert(
            m.material_id,
            ChapterInput {
                material_id: m.material_id,
                version_id: m.version_id,
                title: m.lecture_title.clone(),
                blocks,
            },
        );
        order.push(m.material_id);
    }

    let specs: Vec<UnitSpec> = serde_json::from_value(book.structure.clone()).unwrap_or_default();
    let mut units: Vec<UnitInput> = Vec::new();
    if specs.is_empty() {
        units.push(UnitInput {
            title: None,
            chapters: order.iter().filter_map(|id| by_id.remove(id)).collect(),
        });
    } else {
        for spec in &specs {
            let chapters: Vec<ChapterInput> = spec.material_ids.iter().filter_map(|id| by_id.remove(id)).collect();
            if !chapters.is_empty() {
                units.push(UnitInput { title: spec.title.clone(), chapters });
            }
        }
        let leftover: Vec<ChapterInput> = order.iter().filter_map(|id| by_id.remove(id)).collect();
        if !leftover.is_empty() {
            units.push(UnitInput { title: Some("Other lectures".to_owned()), chapters: leftover });
        }
    }

    let mut front: FrontMatter = serde_json::from_value(book.front_matter.clone()).unwrap_or_default();
    if front.subtitle.is_none() {
        front.subtitle = Some(format!(
            "{} {} \u{b7} {} {}",
            course.code, course.title, course.academic_session, course.semester
        ));
    }
    let (doc, pins) = assemble(&book.title, &front, &units, &settings.language);
    if let Err(problems) = blocks::validate(&doc) {
        return Err(format!("the assembled book is invalid: {}", problems.join("; ")));
    }

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let version_id = Uuid::now_v7();
    let version_no: i32 = sqlx::query_scalar(
        "insert into book_versions (id, book_id, version_no, doc, pins)
         select $1, $2, coalesce(max(version_no), 0) + 1, $3, $4 from book_versions where book_id = $2
         returning version_no",
    )
    .bind(version_id)
    .bind(book.id)
    .bind(serde_json::to_value(&doc).map_err(|e| e.to_string())?)
    .bind(serde_json::to_value(&pins).map_err(|e| e.to_string())?)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    sqlx::query("update course_books set stale = false, chapter_cache = $2, updated_at = now() where id = $1")
        .bind(book.id)
        .bind(Value::Object(fresh_cache))
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok((version_id, version_no, rebuilt, reused))
}

/// Publishes a book version: students see it and `book.published` is emitted.
pub async fn publish(pool: &PgPool, book_id: Uuid, version_id: Uuid) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let course_id: Option<Uuid> = sqlx::query_scalar(
        "update book_versions set published_at = coalesce(published_at, now())
         where id = $1 and book_id = $2 returning (select course_id from course_books where id = $2)",
    )
    .bind(version_id)
    .bind(book_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let course_id = course_id.ok_or_else(|| "book version not found".to_owned())?;
    sqlx::query("update course_books set published_version_id = $2, updated_at = now() where id = $1")
        .bind(book_id)
        .bind(version_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    store::emit(
        &mut *tx,
        "book.published",
        json!({"book_id": book_id, "course_id": course_id, "book_version_id": version_id}),
    )
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chapter(n: u128, title: &str, blocks: Vec<Block>) -> ChapterInput {
        let material_id = Uuid::from_u128(n);
        let mut doc = MaterialDoc::new(title);
        doc.blocks = blocks;
        ChapterInput {
            material_id,
            version_id: Uuid::from_u128(n + 1000),
            title: title.to_owned(),
            blocks: fragment(&doc, material_id),
        }
    }

    fn heading(id: &str, text: &str) -> Block {
        Block::new(id, BlockKind::Heading { level: 1, text: text.into() })
    }
    fn def(id: &str, term: &str) -> Block {
        Block::new(id, BlockKind::Definition { term: term.into(), text: format!("{term} means something") })
    }
    fn eq(id: &str) -> Block {
        Block::new(id, BlockKind::Equation { latex: "a=b".into(), caption: None, number: Some(1) })
    }
    fn key_terms(rows: &[&str]) -> Block {
        Block::new(
            "kt",
            BlockKind::Table {
                caption: Some("Key terms".into()),
                header: vec!["Term".into(), "Where it is discussed".into()],
                rows: rows.iter().map(|t| vec![(*t).to_owned(), "x".to_owned()]).collect(),
            },
        )
    }

    fn two_chapters() -> Vec<UnitInput> {
        vec![UnitInput {
            title: None,
            chapters: vec![
                chapter(1, "Conduction", vec![
                    heading("h", "Fourier's law"),
                    def("d", "Flux"),
                    eq("e"),
                    Block::new("q", BlockKind::ReviewQuestions { questions: vec![ReviewQuestion { question: "Why?".into(), answer: None, difficulty: None }] }),
                    Block::new("r", BlockKind::FurtherReading { items: vec![ReadingItem { title: "Incropera".into(), note: None }] }),
                    key_terms(&["Flux", "Gradient", "Conductivity"]),
                ]),
                chapter(2, "Convection", vec![
                    heading("h", "Newton's law"),
                    def("d", "flux"),
                    def("d2", "Biot number"),
                    eq("e"),
                    Block::new("r", BlockKind::FurtherReading { items: vec![ReadingItem { title: "incropera".into(), note: None }, ReadingItem { title: "Cengel".into(), note: None }] }),
                    key_terms(&["Flux", "Biot number", "Coefficient"]),
                ]),
            ],
        }]
    }

    #[test]
    fn book_has_chapters_back_matter_and_pins() {
        let front = FrontMatter { preface: Some("Welcome.\n\nEnjoy.".into()), ..Default::default() };
        let (doc, pins) = assemble("ME 501 book", &front, &two_chapters(), "English");
        assert_eq!(pins.len(), 2);
        let headings: Vec<String> = doc
            .blocks
            .iter()
            .filter_map(|b| match &b.kind {
                BlockKind::Heading { level, text } => Some(format!("{level}:{text}")),
                _ => None,
            })
            .collect();
        assert_eq!(headings[0], "1:Preface");
        assert!(headings.contains(&"1:Chapter 1: Conduction".to_owned()));
        assert!(headings.contains(&"2:Fourier's law".to_owned()), "{headings:?}");
        assert!(headings.contains(&"1:Chapter 2: Convection".to_owned()));
        for back in ["1:Review questions", "1:Further reading", "1:Glossary", "1:Index"] {
            assert!(headings.contains(&back.to_owned()), "missing {back}");
        }
        assert!(blocks::validate(&doc).is_ok(), "{:?}", blocks::validate(&doc));
    }

    #[test]
    fn repeated_definitions_become_cross_references() {
        let (doc, _) = assemble("B", &FrontMatter::default(), &two_chapters(), "English");
        let recall = doc.blocks.iter().find_map(|b| match &b.kind {
            BlockKind::Callout { text, .. } if text.contains("Recall") => Some(text.clone()),
            _ => None,
        });
        assert_eq!(recall.as_deref(), Some("Recall **flux**, defined in Chapter 1."));
        let glossary = doc.blocks.iter().find(|b| b.id == "back-glossary").unwrap();
        let BlockKind::Table { rows, .. } = &glossary.kind else { panic!() };
        let terms: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
        assert_eq!(terms, vec!["Biot number", "Flux"]);
    }

    #[test]
    fn equations_number_continuously_and_back_matter_aggregates() {
        let (doc, _) = assemble("B", &FrontMatter::default(), &two_chapters(), "English");
        let numbers: Vec<u32> = doc
            .blocks
            .iter()
            .filter_map(|b| match &b.kind {
                BlockKind::Equation { number, .. } => *number,
                _ => None,
            })
            .collect();
        assert_eq!(numbers, vec![1, 2]);
        let questions = doc.blocks.iter().find(|b| b.id == "back-questions").unwrap();
        let BlockKind::ReviewQuestions { questions } = &questions.kind else { panic!() };
        assert_eq!(questions[0].question, "[Chapter 1] Why?");
        let reading = doc.blocks.iter().find(|b| b.id == "back-reading").unwrap();
        let BlockKind::FurtherReading { items } = &reading.kind else { panic!() };
        assert_eq!(items.len(), 2, "duplicate reading is merged case-insensitively");
        let index = doc.blocks.iter().find(|b| b.id == "back-index").unwrap();
        let BlockKind::Table { rows, .. } = &index.kind else { panic!() };
        let flux = rows.iter().find(|r| r[0] == "Flux").unwrap();
        assert_eq!(flux[1], "1, 2");
    }

    #[test]
    fn units_nest_chapters_one_level_deeper() {
        let mut units = two_chapters();
        let second = units[0].chapters.pop().unwrap();
        units.push(UnitInput { title: Some("Unit B".into()), chapters: vec![second] });
        units[0].title = Some("Unit A".into());
        let (doc, _) = assemble("B", &FrontMatter::default(), &units, "English");
        let levels: Vec<(u8, String)> = doc
            .blocks
            .iter()
            .filter_map(|b| match &b.kind {
                BlockKind::Heading { level, text } if text.starts_with("Unit") || text.starts_with("Chapter") => {
                    Some((*level, text.clone()))
                }
                _ => None,
            })
            .collect();
        assert_eq!(levels[0], (1, "Unit A".to_owned()));
        assert_eq!(levels[1], (2, "Chapter 1: Conduction".to_owned()));
        assert_eq!(levels[2], (1, "Unit B".to_owned()));
        assert_eq!(levels[3], (2, "Chapter 2: Convection".to_owned()));
    }

    #[test]
    fn fragments_namespace_ids_and_clear_numbers() {
        let mut doc = MaterialDoc::new("x");
        doc.blocks = vec![eq("e1")];
        let a = fragment(&doc, Uuid::from_u128(1));
        let b = fragment(&doc, Uuid::from_u128(2));
        assert_ne!(a[0].id, b[0].id);
        assert!(matches!(a[0].kind, BlockKind::Equation { number: None, .. }));
    }
}
