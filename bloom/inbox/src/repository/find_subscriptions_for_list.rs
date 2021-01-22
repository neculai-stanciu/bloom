use super::Repository;
use crate::{entities, Error};
use kernel::db::Queryer;
use stdx::sqlx;
use stdx::{log::error, uuid::Uuid};

impl Repository {
    pub async fn find_subscriptions_for_list<'c, C: Queryer<'c>>(
        &self,
        db: C,
        list_id: Uuid,
    ) -> Result<Vec<entities::NewsletterListSubscription>, Error> {
        const QUERY: &str = "SELECT * FROM inbox_newsletter_lists_subscriptions
            WHERE list_id = $2";

        match sqlx::query_as::<_, entities::NewsletterListSubscription>(QUERY)
            .bind(list_id)
            .fetch_all(db)
            .await
        {
            Err(err) => {
                error!("inbox.find_subscriptions_for_list: finding subscriptions: {}", &err);
                Err(err.into())
            }
            Ok(res) => Ok(res),
        }
    }
}