use stdx::chrono::Utc;

use super::{DeleteGroupInput, Service};
use crate::{
    consts::{BillingPlan, GroupRole},
    entities::User,
    errors::kernel::Error,
};

impl Service {
    pub async fn delete_group(&self, actor: Option<User>, input: DeleteGroupInput) -> Result<(), crate::Error> {
        let actor = self.current_user(actor)?;

        let group = self.repo.find_group_by_id(&self.db, input.group_id).await?;
        let namespace = self.repo.find_namespace_by_id(&self.db, group.namespace_id).await?;

        // check that user is admin
        let membership = self.repo.find_group_membership(&self.db, group.id, actor.id).await?;
        if membership.role != GroupRole::Administrator {
            return Err(Error::AdminRoleRequired.into());
        }

        if group.plan != BillingPlan::Free {
            return Err(Error::SubscriptionIsActive.into());
        }

        let customer = self
            .repo
            .find_customer_by_namespace_id(&self.db, namespace.id)
            .await
            .ok();

        let mut tx = self.db.begin().await?;

        if let Some(mut customer) = customer {
            customer.namespace_id = None;
            customer.updated_at = Utc::now();
            self.repo.update_customer(&mut tx, &customer).await?;
        }

        self.repo.delete_namespace(&mut tx, group.namespace_id).await?;

        tx.commit().await?;

        Ok(())
    }
}