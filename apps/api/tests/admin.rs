use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

use lumiforum_api::models::{
    AuthenticatedPrincipal, PERMISSION_ADMIN_ACCESS, PERMISSION_USER_MANAGE,
};
use lumiforum_api::services::AdminError;

#[tokio::test]
async fn admin_permission_gate_and_user_list_require_admin_access() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(());
    };
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let principal = AuthenticatedPrincipal::new(
        Uuid::new_v4(),
        "user".into(),
        0,
        Uuid::new_v4(),
        [PERMISSION_USER_MANAGE.to_owned()],
    );
    assert!(
        !principal.has_permission(PERMISSION_ADMIN_ACCESS),
        "regular user manage permission alone must not imply admin.access"
    );

    let admin_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT count(*)
        FROM role_permissions rp
        JOIN roles r ON r.id = rp.role_id
        JOIN permissions p ON p.id = rp.permission_id
        WHERE p.code = 'admin.access'
          AND r.code IN ('administrator', 'super_administrator')
        "#,
    )
    .fetch_one(&pool)
    .await?;
    assert!(admin_count >= 2);

    let guest_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT count(*)
        FROM role_permissions rp
        JOIN roles r ON r.id = rp.role_id
        JOIN permissions p ON p.id = rp.permission_id
        WHERE p.code = 'admin.access' AND r.code IN ('user', 'moderator')
        "#,
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(guest_count, 0);

    let _ = AdminError::Forbidden;
    Ok(())
}
