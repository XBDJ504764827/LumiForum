use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, Params, PasswordHasher,
};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;
    let password = "AdminTest123!";
    let hash = hash_password(password)?;

    for (username, email, role, nickname) in [
        (
            "admin_test",
            "admin_test@lumiforum.local",
            "administrator",
            "Admin Tester",
        ),
        (
            "super_admin_test",
            "super_admin_test@lumiforum.local",
            "super_administrator",
            "Super Admin Tester",
        ),
    ] {
        let updated = sqlx::query(
            r#"
            WITH upsert AS (
                INSERT INTO users (
                    username, email, password_hash, nickname, role_id, status,
                    email_verified, email_verified_at
                )
                SELECT $1, $2, $3, $4, roles.id, 'active', true, now()
                FROM roles
                WHERE roles.code = $5
                ON CONFLICT DO NOTHING
                RETURNING id
            )
            UPDATE users
            SET password_hash = $3,
                nickname = $4,
                status = 'active',
                email_verified = true,
                email_verified_at = COALESCE(users.email_verified_at, now()),
                role_id = (SELECT id FROM roles WHERE code = $5),
                auth_version = users.auth_version + 1
            WHERE NOT EXISTS (SELECT 1 FROM upsert)
              AND (lower(username) = lower($1) OR lower(email) = lower($2))
            "#,
        )
        .bind(username)
        .bind(email)
        .bind(&hash)
        .bind(nickname)
        .bind(role)
        .execute(&pool)
        .await?;

        if updated.rows_affected() > 0 {
            println!("updated: {username} ({role})");
        } else {
            println!("created: {username} ({role})");
        }
    }

    println!("password: {password}");
    Ok(())
}

fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let params =
        Params::new(19_456, 2, 1, None).map_err(|_| anyhow::anyhow!("invalid argon params"))?;
    let argon = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    argon
        .hash_password(password.as_bytes(), &salt)
        .map(|value| value.to_string())
        .map_err(|_| anyhow::anyhow!("failed to hash password"))
}
