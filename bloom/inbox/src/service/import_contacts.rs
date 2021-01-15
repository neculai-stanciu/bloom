use super::ImportContactsInput;
use crate::{
    consts,
    entities::{Contact, ImportedContact, NewsletterListContactRelation},
    Error, Service,
};
use kernel::Actor;
use std::collections::HashMap;
use stdx::{chrono::Utc, csv, ulid::Ulid};

impl Service {
    // TODO: check if list/contact relation exists
    pub async fn import_contacts(
        &self,
        actor: Actor,
        input: ImportContactsInput,
    ) -> Result<Vec<Contact>, kernel::Error> {
        let actor = self.kernel_service.current_user(actor)?;

        let namespace_id = input.namespace_id;

        self.kernel_service
            .check_namespace_membership(&self.db, actor.id, namespace_id)
            .await?;

        let list = if let Some(list_id) = input.list_id {
            let list = self.repo.find_newsletter_list_by_id(&self.db, list_id).await?;
            Some(list)
        } else {
            None
        };

        if let Some(ref list) = list {
            if list.namespace_id != namespace_id {
                return Err(Error::PermissionDenied.into());
            }
        }

        if input.contacts_csv.len() > consts::MAX_IMPORT_CONTACTS_CSV_LENGTH {
            return Err(Error::ContactsCsvTooLarge.into());
        }

        let mut imported_contacts: Vec<ImportedContact> = Vec::with_capacity(500);
        let mut csv_reader = csv::Reader::from_reader(input.contacts_csv.as_bytes());
        for record in csv_reader.deserialize() {
            let imported_contact: ImportedContact = record?;
            imported_contacts.push(imported_contact);
        }

        // dedup
        let unique_contacts: HashMap<String, ImportedContact> = imported_contacts
            .into_iter()
            .map(|contact| ImportedContact {
                name: contact.name.trim().to_string(),
                email: contact.email.trim().to_lowercase(),
            })
            .map(|contact| (contact.email.clone(), contact))
            .collect();
        let unique_contacts: Vec<ImportedContact> = unique_contacts
            .into_iter()
            .map(|entry| entry.1)
            .filter(|contact| !contact.email.is_empty())
            .collect();

        for contact in &unique_contacts {
            self.kernel_service.validate_email(&contact.email, false)?;
            self.validate_contact_name(&contact.name)?;
        }

        let mut contacts: Vec<Contact> = Vec::with_capacity(unique_contacts.len());
        let now = Utc::now();

        let mut tx = self.db.begin().await?;

        for contact in unique_contacts {
            let res = self.repo.find_contact_by_email(&mut tx, &contact.email).await;
            let contact = match res {
                Ok(mut existing_contact) => {
                    // update contact, maybe
                    if !contact.name.is_empty() && contact.name != existing_contact.name {
                        existing_contact.updated_at = now;
                        existing_contact.name = contact.name;
                        self.repo.update_contact(&mut tx, &existing_contact).await?;
                    }

                    Ok(existing_contact)
                }
                Err(Error::ContactNotFound) => {
                    // create contact
                    let new_contact = Contact {
                        id: Ulid::new().into(),
                        created_at: now,
                        updated_at: now,
                        name: contact.name,
                        birthday: None,
                        email: contact.email,
                        pgp_key: String::new(),
                        phone: String::new(),
                        address: String::new(),
                        website: String::new(),
                        twitter: String::new(),
                        instagram: String::new(),
                        facebook: String::new(),
                        linkedin: String::new(),
                        skype: String::new(),
                        telegram: String::new(),
                        bloom: String::new(),
                        notes: String::new(),
                        plan: String::new(),
                        user_id: String::new(),
                        country: String::new(),
                        country_code: String::new(),
                        namespace_id,
                        avatar_storage_key: None,
                    };
                    self.repo.create_contact(&mut tx, &new_contact).await?;

                    Ok(new_contact)
                }
                Err(err) => Err(err),
            }?;

            if let Some(ref list) = list {
                let list_contact_relation = NewsletterListContactRelation {
                    list_id: list.id,
                    contact_id: contact.id,
                };
                self.repo
                    .create_newsletter_list_contact_relation(&mut tx, &list_contact_relation)
                    .await?;
            }

            contacts.push(contact);
        }

        tx.commit().await?;

        Ok(contacts)
    }
}
