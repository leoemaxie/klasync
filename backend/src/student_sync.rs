//! Matches student accounts to the lecture sessions they attended.
//!
//! Guests join a session with only a matriculation number, so a session's
//! participant rows and a student's account are linked *by matric number*.
//! Historically that link was only created when the student pressed "claim" on
//! the very device they joined from, which meant a different device (or a
//! student who registered after the lecture) saw an empty or partial archive.
//!
//! `reconcile_student` makes the link server-side and idempotent so every
//! login on every device converges on the same archive and course list.
//!
//! These queries intentionally use the runtime `sqlx::query` API so they do not
//! need regenerating in `.sqlx`.

use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Default, Clone, Copy)]
pub struct ReconcileOutcome {
    pub claims_created: u64,
    pub participants_linked: u64,
    pub enrollments_created: u64,
}

/// Link every participant row whose matric number matches `student_id`'s
/// matric number, then enroll the student in the matching courses.
pub async fn reconcile_student(
    pool: &PgPool,
    student_id: Uuid,
) -> Result<ReconcileOutcome, sqlx::Error> {
    // Participants are only matched when nobody else already owns the row.
    let claims_created = sqlx::query(
        "insert into student_session_claims (participant_id, student_account_id, verified_at)
         select p.id, sa.id, now()
         from student_accounts sa
         join session_participants p on lower(trim(p.matric_number)) = lower(trim(sa.matric_number))
         where sa.id = $1
           and (p.student_account_id is null or p.student_account_id = sa.id)
         on conflict (participant_id) do nothing",
    )
    .bind(student_id)
    .execute(pool)
    .await?
    .rows_affected();

    let participants_linked = sqlx::query(
        "update session_participants p
         set student_account_id = sa.id
         from student_accounts sa
         where sa.id = $1
           and p.student_account_id is null
           and lower(trim(p.matric_number)) = lower(trim(sa.matric_number))",
    )
    .bind(student_id)
    .execute(pool)
    .await?
    .rows_affected();

    // Courses of every session the student attended.
    let from_claims = sqlx::query(
        "insert into student_course_enrollments (student_account_id, course_id, enrollment_type)
         select distinct c.student_account_id, s.course_id, 'claimed'
         from student_session_claims c
         join session_participants p on p.id = c.participant_id
         join lecture_sessions s on s.id = p.session_id
         where c.student_account_id = $1
         on conflict (student_account_id, course_id) do nothing",
    )
    .bind(student_id)
    .execute(pool)
    .await?
    .rows_affected();

    // Courses whose roster lists the student, even if they missed every lecture.
    let from_roster = sqlx::query(
        "insert into student_course_enrollments (student_account_id, course_id, enrollment_type)
         select distinct sa.id, r.course_id, 'roster_matched'
         from student_accounts sa
         join roster_students r on lower(trim(r.matric_number)) = lower(trim(sa.matric_number))
         where sa.id = $1
         on conflict (student_account_id, course_id) do nothing",
    )
    .bind(student_id)
    .execute(pool)
    .await?
    .rows_affected();

    Ok(ReconcileOutcome {
        claims_created,
        participants_linked,
        enrollments_created: from_claims + from_roster,
    })
}

/// Best-effort wrapper for request handlers: reconciliation problems must never
/// take down the page that triggered them.
pub async fn reconcile_student_logged(pool: &PgPool, student_id: Uuid) {
    match reconcile_student(pool, student_id).await {
        Ok(outcome) => {
            if outcome.claims_created + outcome.enrollments_created > 0 {
                tracing::info!(
                    %student_id,
                    claims = outcome.claims_created,
                    enrollments = outcome.enrollments_created,
                    "Reconciled student archive"
                );
            }
        }
        Err(error) => tracing::warn!(%error, %student_id, "Student archive reconciliation failed"),
    }
}

/// A student may read a session's study material when they attended it (a
/// claim or a linked participant row), or the session belongs to a course whose
/// roster lists them and the lecture has finished.
pub async fn student_can_access_session(
    pool: &PgPool,
    student_id: Uuid,
    session_id: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "select exists(
            select 1 from session_participants p
            where p.session_id = $2
              and (p.student_account_id = $1
                   or exists(select 1 from student_session_claims c
                             where c.participant_id = p.id and c.student_account_id = $1))
         ) or exists(
            select 1
            from lecture_sessions s
            join student_course_enrollments e on e.course_id = s.course_id
            where s.id = $2 and e.student_account_id = $1
              and e.enrollment_type in ('claimed', 'roster_matched')
              and s.status = 'ended'
         )",
    )
    .bind(student_id)
    .bind(session_id)
    .fetch_one(pool)
    .await
}
