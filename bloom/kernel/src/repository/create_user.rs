use super::Repository;
use crate::{db, entities, errors::kernel::Error};

impl Repository {
    pub async fn create_user<'c, C: db::Queryer<'c>>(&self, db: C, user: &entities::User) -> Result<(), Error> {
        const QUERY: &str = "NSERT INTO kernel_users
            (id, created_at, updated_at, blocked_at, username, email, is_admin, two_fa_enabled, two_fa_method, encrypted_totp_secret,
                totp_secret_nonce, name, description, avatar, used_storage, plan, namespace_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)";

        match sqlx::query(QUERY)
            .bind(user.id)
            .bind(user.created_at)
            .bind(user.updated_at)
            .bind(user.blocked_at)
            .bind(&user.username)
            .bind(&user.email)
            .bind(user.is_admin)
            .bind(user.two_fa_enabled)
            .bind(user.two_fa_method)
            .bind(&user.encrypted_totp_secret)
            .bind(&user.totp_secret_nonce)
            .bind(&user.name)
            .bind(&user.description)
            .bind(&user.avatar)
            .bind(user.used_storage)
            .bind(user.plan)
            .bind(user.namespace_id)
            .execute(db)
            .await
        {
            Err(err) => {
                println!("kernel.create_user: Inserting user: {}", &err);
                Err(err.into())
            }
            Ok(_) => Ok(()),
        }
    }
}