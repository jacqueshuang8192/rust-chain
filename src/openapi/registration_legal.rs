//! 注册法律文件的公开版本、管理员发布和确认凭据合同。

use super::*;

#[derive(ToSchema)]
struct LegalDocument {
    id: u64,
    title: String,
    body: LegalBody,
    effective_on: String,
}

#[derive(ToSchema)]
#[serde(untagged)]
enum LegalBody {
    Plain(String),
    Rich(Vec<LegalTextBlock>),
}

#[derive(ToSchema)]
struct LegalTextBlock {
    #[serde(rename = "type")]
    kind: LegalBlockType,
    children: Vec<LegalTextLeaf>,
}

#[derive(ToSchema)]
#[serde(rename_all = "lowercase")]
enum LegalBlockType {
    P,
    H1,
    H2,
    H3,
    Blockquote,
}

#[derive(ToSchema)]
struct LegalTextLeaf {
    text: String,
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<bool>,
}

#[derive(ToSchema)]
struct LegalBundle {
    publication_id: u64,
    published_at: String,
    scope_code: String,
    locale: String,
    edition: u64,
    terms: LegalDocument,
    privacy: LegalDocument,
}

#[derive(ToSchema)]
struct LegalEdition {
    id: u64,
    edition: u64,
    published_at: String,
    published_by: u64,
}

#[derive(ToSchema)]
struct AdminLegalResponse {
    current: Option<LegalBundle>,
    editions: Vec<LegalEdition>,
}

#[derive(ToSchema)]
struct PublishLegalRequest {
    scope_code: String,
    locale: String,
    expected_edition: u64,
    effective_on: String,
    terms_title: String,
    terms_body: LegalBody,
    privacy_title: String,
    privacy_body: LegalBody,
    reason: String,
}

#[derive(ToSchema)]
struct LegalReceipt {
    user_id: u64,
    country_code: String,
    scope_code: String,
    locale: String,
    publication_id: u64,
    terms_id: u64,
    privacy_id: u64,
    acknowledged_at: String,
}

/// 匿名读取实际适用的一组协议，只返回已经发布的正文。
#[utoipa::path(get, path="/api/v1/auth/register/legal", tag="auth",
    params(("country_code"=String, Query, description="现有注册国家代码"), ("locale"=Option<String>, Query, description="界面语言")),
    responses((status=200, body=LegalBundle), (status=400, body=ErrorResponse),
        (status=409, description="未发布完整协议组", body=ErrorResponse)))]
fn public_registration_legal() {}

/// 后台精确地区/语言查询当前发布与近期版本，不使用公开回退结果。
#[utoipa::path(get, path="/admin/api/v1/legal-documents", tag="admin-legal",
    params(("scope_code"=String, Query), ("locale"=String, Query)), security(("bearerAuth"=[])),
    responses((status=200, body=AdminLegalResponse), (status=403, body=ErrorResponse)))]
fn admin_legal_index() {}

/// 原子发布两份不可变正文，并记录管理员原因与新版本主键。
#[utoipa::path(post, path="/admin/api/v1/legal-documents", tag="admin-legal",
    request_body=PublishLegalRequest, security(("bearerAuth"=[])),
    responses((status=200, body=LegalBundle), (status=400, body=ErrorResponse),
        (status=403, body=ErrorResponse), (status=409, body=ErrorResponse)))]
fn publish_registration_legal() {}

/// 只向有权限的管理员返回已发布的旧版正文。
#[utoipa::path(get, path="/admin/api/v1/legal-documents/publications/{id}", tag="admin-legal",
    params(("id"=u64, Path)), security(("bearerAuth"=[])),
    responses((status=200, body=LegalBundle), (status=403, body=ErrorResponse), (status=404, body=ErrorResponse)))]
fn admin_legal_publication() {}

/// 按用户主键查询注册时的不可变确认记录，不暴露给匿名用户。
#[utoipa::path(get, path="/admin/api/v1/legal-documents/receipts/{user_id}", tag="admin-legal",
    params(("user_id"=u64, Path)), security(("bearerAuth"=[])),
    responses((status=200, body=LegalReceipt), (status=403, body=ErrorResponse), (status=404, body=ErrorResponse)))]
fn admin_legal_receipt() {}

#[derive(OpenApi)]
#[openapi(
    paths(
        public_registration_legal,
        admin_legal_index,
        publish_registration_legal,
        admin_legal_publication,
        admin_legal_receipt
    ),
    components(schemas(
        LegalDocument,
        LegalBody,
        LegalTextBlock,
        LegalBlockType,
        LegalTextLeaf,
        LegalBundle,
        LegalEdition,
        AdminLegalResponse,
        PublishLegalRequest,
        LegalReceipt
    ))
)]
pub(super) struct RegistrationLegalApiDoc;
