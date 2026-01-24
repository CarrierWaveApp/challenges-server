use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{
    Challenge, ChallengeListItem, CreateChallengeRequest, ListChallengesQuery,
};

pub async fn list_challenges(
    pool: &PgPool,
    query: &ListChallengesQuery,
) -> Result<(Vec<ChallengeListItem>, i64), AppError> {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let challenges = sqlx::query_as!(
        ChallengeListItem,
        r#"
        SELECT
            c.id,
            c.name,
            c.description,
            c.category,
            c.challenge_type,
            c.is_active,
            COALESCE(COUNT(cp.id), 0) as "participant_count!"
        FROM challenges c
        LEFT JOIN challenge_participants cp ON cp.challenge_id = c.id AND cp.status = 'active'
        WHERE ($1::text IS NULL OR c.category = $1)
          AND ($2::text IS NULL OR c.challenge_type = $2)
          AND ($3::bool IS NULL OR c.is_active = $3)
        GROUP BY c.id
        ORDER BY c.created_at DESC
        LIMIT $4 OFFSET $5
        "#,
        query.category,
        query.challenge_type,
        query.active,
        limit,
        offset,
    )
    .fetch_all(pool)
    .await?;

    let total = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) as "count!"
        FROM challenges c
        WHERE ($1::text IS NULL OR c.category = $1)
          AND ($2::text IS NULL OR c.challenge_type = $2)
          AND ($3::bool IS NULL OR c.is_active = $3)
        "#,
        query.category,
        query.challenge_type,
        query.active,
    )
    .fetch_one(pool)
    .await?;

    Ok((challenges, total))
}

pub async fn get_challenge(pool: &PgPool, id: Uuid) -> Result<Option<Challenge>, AppError> {
    let challenge = sqlx::query_as!(
        Challenge,
        r#"
        SELECT
            id, version, name, description, author, category, challenge_type,
            configuration, invite_config, hamalert_config, is_active,
            created_at, updated_at
        FROM challenges
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(challenge)
}

pub async fn create_challenge(
    pool: &PgPool,
    req: &CreateChallengeRequest,
) -> Result<Challenge, AppError> {
    let id = Uuid::new_v4();

    let challenge = sqlx::query_as!(
        Challenge,
        r#"
        INSERT INTO challenges (id, name, description, author, category, challenge_type, configuration, invite_config, hamalert_config)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, version, name, description, author, category, challenge_type,
                  configuration, invite_config, hamalert_config, is_active,
                  created_at, updated_at
        "#,
        id,
        req.name,
        req.description,
        req.author,
        req.category,
        req.challenge_type,
        req.configuration,
        req.invite_config,
        req.hamalert_config,
    )
    .fetch_one(pool)
    .await?;

    Ok(challenge)
}

pub async fn update_challenge(
    pool: &PgPool,
    id: Uuid,
    req: &CreateChallengeRequest,
) -> Result<Option<Challenge>, AppError> {
    let challenge = sqlx::query_as!(
        Challenge,
        r#"
        UPDATE challenges
        SET name = $2, description = $3, author = $4, category = $5,
            challenge_type = $6, configuration = $7, invite_config = $8,
            hamalert_config = $9, version = version + 1, updated_at = now()
        WHERE id = $1
        RETURNING id, version, name, description, author, category, challenge_type,
                  configuration, invite_config, hamalert_config, is_active,
                  created_at, updated_at
        "#,
        id,
        req.name,
        req.description,
        req.author,
        req.category,
        req.challenge_type,
        req.configuration,
        req.invite_config,
        req.hamalert_config,
    )
    .fetch_optional(pool)
    .await?;

    Ok(challenge)
}

pub async fn delete_challenge(pool: &PgPool, id: Uuid) -> Result<bool, AppError> {
    let result = sqlx::query!("DELETE FROM challenges WHERE id = $1", id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}
