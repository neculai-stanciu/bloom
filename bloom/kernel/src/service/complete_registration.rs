use consts::{BillingPlan, NamespaceType};
use stdx::{
    chrono::{Duration, Utc},
    crypto,
    rand::{thread_rng, Rng},
    sync::threadpool::spawn_blocking,
    ulid::Ulid,
};
use tokio::time::delay_for;

use super::{CompleteRegistrationInput, CreateNamespaceInput, Service, SignedIn};
use crate::{
    consts,
    entities::{self, User},
    errors::kernel::Error,
};

impl Service {
    pub async fn complete_registration(
        &self,
        actor: Option<User>,
        input: CompleteRegistrationInput,
    ) -> Result<SignedIn, crate::Error> {
        if actor.is_some() {
            return Err(Error::MustNotBeAuthenticated.into());
        }

        // sleep to prevent spam and bruteforce
        let sleep = thread_rng().gen_range(consts::SLEEP_MIN..consts::SLEEP_MAX);
        delay_for(sleep).await;

        let mut pending_user = self
            .repo
            .find_pending_user_by_id(&self.db, input.pending_user_id)
            .await?;

        if pending_user.failed_attempts >= consts::REGISTRATION_MAX_FAILED_ATTEMPTS {
            return Err(Error::MaxRegistrationAttempsReached.into());
        }

        let code = input.code.trim().to_lowercase().replace("-", "");
        let now = Utc::now();

        let created_at_plus_30_mins = pending_user.created_at + Duration::minutes(30);
        if now >= created_at_plus_30_mins {
            return Err(Error::RegistrationCodeExpired.into());
        }

        let code_hash = pending_user.code_hash.clone();
        let is_code_valid = spawn_blocking(move || crypto::verify_password(&code, &code_hash)).await?;

        if !is_code_valid {
            pending_user.failed_attempts += 1;
            let _ = self.repo.update_pending_user(&self.db, &pending_user).await;
            return Err(Error::InvalidRegistrationCode.into());
        }

        let mut tx = self.db.begin().await?;

        let find_existing_user_res = self.repo.find_user_by_email(&mut tx, &pending_user.email).await;
        match find_existing_user_res {
            Ok(_) => Err(Error::EmailAlreadyExists),
            Err(Error::UserNotFound) => Ok(()),
            Err(err) => Err(err),
        }?;

        let namespace_exists = self.check_namespace_exists(&mut tx, &pending_user.username).await?;
        if namespace_exists {
            return Err(Error::UsernameAlreadyExists.into());
        }

        self.repo.delete_pending_user(&mut tx, pending_user.id).await?;

        let create_namespace_input = CreateNamespaceInput {
            path: pending_user.username.clone(),
            namespace_type: NamespaceType::User,
        };
        let namespace = self.create_namespace(&mut tx, create_namespace_input).await?;

        let users_count = self.repo.get_users_count(&mut tx).await?;

        // create a new user, session and delete pending user
        let now = Utc::now();
        let user = entities::User {
            id: Ulid::new().into(),
            created_at: now,
            updated_at: now,
            blocked_at: None,
            username: pending_user.username.clone(),
            email: pending_user.email,
            is_admin: users_count == 0,
            two_fa_enabled: false,
            two_fa_method: None,
            encrypted_totp_secret: None,
            totp_secret_nonce: None,
            name: pending_user.username,
            description: String::new(),
            used_storage: 0,
            plan: BillingPlan::Free,
            avatar: None,
            namespace_id: namespace.id,
        };
        let session = self.new_session(user.id).await?;

        self.repo.create_user(&mut tx, &user).await?;
        self.repo.create_session(&mut tx, &session).await?;

        tx.commit().await?;

        Ok(SignedIn::Success { session, user })
    }
}