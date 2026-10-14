use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct RegistrationLegalDocument {
    pub id: u64,
    pub title: String,
    pub body: String,
    pub effective_on: NaiveDate,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegistrationLegalBundle {
    pub publication_id: u64,
    pub published_at: chrono::DateTime<chrono::Utc>,
    pub scope_code: String,
    pub locale: String,
    pub edition: u64,
    pub terms: RegistrationLegalDocument,
    pub privacy: RegistrationLegalDocument,
}

#[derive(Debug, Deserialize)]
pub struct RegistrationLegalQuery {
    pub country_code: String,
    pub locale: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminLegalQuery {
    pub scope_code: String,
    pub locale: String,
}

#[derive(Debug, Deserialize)]
pub struct PublishRegistrationLegalRequest {
    pub scope_code: String,
    pub locale: String,
    pub expected_edition: u64,
    pub effective_on: NaiveDate,
    pub terms_title: String,
    pub terms_body: String,
    pub privacy_title: String,
    pub privacy_body: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct AdminLegalResponse {
    pub current: Option<RegistrationLegalBundle>,
    pub editions: Vec<RegistrationLegalEdition>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct RegistrationLegalEdition {
    pub id: u64,
    pub edition: u64,
    pub published_at: chrono::DateTime<chrono::Utc>,
    pub published_by: u64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserRegistrationLegalReceipt {
    pub user_id: u64,
    pub country_code: String,
    pub scope_code: String,
    pub locale: String,
    pub publication_id: u64,
    pub terms_id: u64,
    pub privacy_id: u64,
    pub acknowledged_at: chrono::DateTime<chrono::Utc>,
}
