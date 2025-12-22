// src/models.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RightsAgreementJSON {
    pub agreement_id: String,
    pub rights_holder: RightsHolder,
    pub content: ContentInfo,
    pub rights: Rights,
    pub financial: Financial,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parties: Option<Parties>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deliverables: Option<Deliverables>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restrictions: Option<Restrictions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub special_terms: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legal_terms: Option<LegalTerms>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RightsHolder {
    pub name: String,
    pub wallet_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentInfo {
    pub title: String,
    pub original_title: String,
    #[serde(rename = "type")]
    pub content_type: String,
    pub language: String,
    pub genre: Vec<String>,
    pub duration: u32,
    pub release_date: String,
    pub director: String,
    pub producer: String,
    pub rating: Rating,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rating {
    pub cbfc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mpaa: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rights {
    pub territories: Vec<String>,
    pub media_types: Vec<String>,
    pub exclusivity: bool,
    pub term: Term,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Term {
    pub years: u32,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Financial {
    pub deal_value: u64,
    pub currency: String,
    pub platform_fee: PlatformFee,
    pub net_to_rights_holder: u64,
    pub payment_structure: PaymentStructure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformFee {
    pub percentage: f64,
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentStructure {
    #[serde(rename = "type")]
    pub payment_type: String,
    pub breakdown: PaymentBreakdown,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub milestones: Option<Vec<Milestone>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentBreakdown {
    pub upfront: u64,
    #[serde(rename = "onDelivery")]
    pub on_delivery: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Milestone {
    pub name: String,
    pub amount: u64,
    pub due_date: String,
    pub percentage: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parties {
    pub licensor: Party,
    pub licensee: Party,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Party {
    pub name: String,
    pub registration_number: String,
    pub address: String,
    pub country: String,
    pub contact_email: String,
    pub signatory_name: String,
    pub signatory_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deliverables {
    pub video_formats: Vec<String>,
    pub audio_formats: Vec<String>,
    pub subtitles: Vec<String>,
    pub dubbing: Vec<String>,
    pub delivery_deadline: String,
    pub technical_specs: TechnicalSpecs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TechnicalSpecs {
    pub video_codec: String,
    pub audio_codec: String,
    pub container_format: String,
    pub drm_required: bool,
    pub drm_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Restrictions {
    pub territories_excluded: Vec<String>,
    pub platforms_excluded: Vec<String>,
    pub holdback_period: HoldbackPeriod,
    pub content_rating: String,
    pub editing_rights: String,
    pub merchandising_rights: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoldbackPeriod {
    pub theatrical: u32,
    #[serde(rename = "physicalMedia")]
    pub physical_media: u32,
    #[serde(rename = "freeTV")]
    pub free_tv: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegalTerms {
    pub governing_law: String,
    pub dispute_resolution: String,
    pub confidentiality: String,
    pub warranties: String,
    pub indemnification: String,
    pub forcemajeure: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub created_date: String,
    pub last_modified: String,
    pub version: String,
    pub status: String,
    pub blockchain: BlockchainInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockchainInfo {
    pub network: String,
    pub deployment_pending: bool,
}

// LLM Response Structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedAgreement {
    pub title: String,
    pub licensor: String,
    pub licensee: String,
    pub territories: Vec<String>,
    pub media_types: Vec<String>,
    pub deal_value: u64,
    pub currency: String,
    pub term_years: Option<u32>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub exclusivity: bool,
    pub content_type: Option<String>,
    pub language: Option<String>,
    pub genre: Vec<String>,
    pub director: Option<String>,
    pub producer: Option<String>,
    pub release_date: Option<String>,
    pub duration: Option<u32>,
}