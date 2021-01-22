use stdx::{
    chrono::{DateTime, Utc},
    sqlx,
    uuid::Uuid,
};

use crate::consts;

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct File {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    pub name: String,
    pub size: i64,
    pub r#type: String,
    pub explicitly_trashed: bool,
    pub trashed_at: Option<DateTime<Utc>>,

    pub namespace_id: Option<Uuid>,
    pub parent_id: Option<Uuid>,
}

impl File {
    pub fn is_root(&self) -> bool {
        return self.name == consts::ROOT_FILE_NAME;
    }

    pub fn storage_key(&self) -> String {
        // TODO: improve
        let id_str = self.id.to_hyphenated().to_string();
        format!("/files/{}/{}", &id_str[..4], &id_str)
    }
}

#[cfg(test)]
mod tests {
    use super::File;
    use stdx::{chrono::Utc, uuid::Uuid};

    #[test]
    fn file_storage_key() {
        let id = "c2ae4298-48a2-478b-a9f2-5eef5d9b54cd".parse::<Uuid>().unwrap();
        let now = Utc::now();
        let expected_storage_key = "/files/c2ae/c2ae4298-48a2-478b-a9f2-5eef5d9b54cd".to_string();

        let file = File {
            id,
            created_at: now,
            updated_at: now,

            name: String::new(),
            size: 0,
            r#type: String::new(),
            explicitly_trashed: false,
            trashed_at: None,

            namespace_id: None,
            parent_id: None,
        };

        assert_eq!(file.storage_key(), expected_storage_key);
    }
}
