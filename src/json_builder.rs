// src/json_builder.rs
use anyhow::Result;
use chrono::Utc;
use tracing::info;
use crate::models::*;

pub struct JSONBuilder;

impl JSONBuilder {
    pub fn new() -> Self {
        Self
    }

    pub async fn build_agreement(&self, parsed: &ParsedAgreement) -> Result<RightsAgreementJSON> {
        info!("🔨 Building JSON structure");

        // Generate agreement ID
        let title = &parsed.title;
        let agreement_id = format!("{}-{}-{}", 
            parsed.licensor.replace(" ", "-").to_uppercase(),
            title.split_whitespace().next().unwrap_or("TITLE"),
            Utc::now().format("%Y")
        );

        // Calculate financial details
        let platform_fee_percentage = 2.5;
        let platform_fee_amount = (parsed.deal_value as f64 * platform_fee_percentage / 100.0) as u64;
        let net_to_holder = parsed.deal_value - platform_fee_amount;

        // Build complete structure
        let agreement = RightsAgreementJSON {
            agreement_id,
            rights_holder: RightsHolder {
                name: parsed.licensor.clone(),
                wallet_address: "0x0000000000000000000000000000000000000000".to_string(), // To be filled
            },
            content: ContentInfo {
                title: parsed.title.clone(),
                original_title: parsed.title.clone(),
                content_type: parsed.content_type.clone().unwrap_or_else(|| "MOVIE".to_string()),
                language: parsed.language.clone().unwrap_or_else(|| "Unknown".to_string()),
                genre: parsed.genre.clone(),
                duration: parsed.duration.unwrap_or(120),
                release_date: parsed.release_date.clone().unwrap_or_else(|| "Unknown".to_string()),
                director: parsed.director.clone().unwrap_or_else(|| "Unknown".to_string()),
                producer: parsed.producer.clone().unwrap_or_else(|| "Unknown".to_string()),
                rating: Rating {
                    cbfc: "U/A".to_string(),
                    mpaa: Some("PG-13".to_string()),
                },
            },
            rights: Rights {
                territories: parsed.territories.clone(),
                media_types: parsed.media_types.clone(),
                exclusivity: parsed.exclusivity,
                term: Term {
                    years: parsed.term_years.unwrap_or(1),
                    start_date: parsed.start_date.clone().unwrap_or_else(|| "Unknown".to_string()),
                    end_date: parsed.end_date.clone().unwrap_or_else(|| "Unknown".to_string()),
                },
            },
            financial: Financial {
                deal_value: parsed.deal_value,
                currency: parsed.currency.clone(),
                platform_fee: PlatformFee {
                    percentage: platform_fee_percentage,
                    amount: platform_fee_amount,
                },
                net_to_rights_holder: net_to_holder,
                payment_structure: PaymentStructure {
                    payment_type: "FIXED".to_string(),
                    breakdown: PaymentBreakdown {
                        upfront: parsed.deal_value / 2,
                        on_delivery: parsed.deal_value / 2,
                    },
                    milestones: None,
                },
            },
            parties: Some(Parties {
                licensor: Party {
                    name: parsed.licensor.clone(),
                    registration_number: "TBD".to_string(),
                    address: "TBD".to_string(),
                    country: "TBD".to_string(),
                    contact_email: "contact@licensor.com".to_string(),
                    signatory_name: "TBD".to_string(),
                    signatory_title: "CEO".to_string(),
                },
                licensee: Party {
                    name: parsed.licensee.clone(),
                    registration_number: "TBD".to_string(),
                    address: "TBD".to_string(),
                    country: "TBD".to_string(),
                    contact_email: "contact@licensee.com".to_string(),
                    signatory_name: "TBD".to_string(),
                    signatory_title: "CEO".to_string(),
                },
            }),
            deliverables: Some(Deliverables {
                video_formats: vec![
                    "4K_UHD".to_string(),
                    "HD_1080p".to_string(),
                    "HD_720p".to_string(),
                ],
                audio_formats: vec![
                    "5.1_Surround".to_string(),
                    "Stereo".to_string(),
                ],
                subtitles: vec![
                    "English".to_string(),
                    "Hindi".to_string(),
                ],
                dubbing: vec![
                    "Hindi".to_string(),
                ],
                delivery_deadline: "2025-03-01".to_string(),
                technical_specs: TechnicalSpecs {
                    video_codec: "H.265/HEVC".to_string(),
                    audio_codec: "AAC".to_string(),
                    container_format: "MP4".to_string(),
                    drm_required: true,
                    drm_type: "Widevine, PlayReady".to_string(),
                },
            }),
            restrictions: None,
            special_terms: None,
            legal_terms: Some(LegalTerms {
                governing_law: "Laws of India".to_string(),
                dispute_resolution: "Arbitration".to_string(),
                confidentiality: "5 years".to_string(),
                warranties: "Standard warranties apply".to_string(),
                indemnification: "Mutual indemnification".to_string(),
                forcemajeure: "Standard force majeure clause".to_string(),
            }),
            metadata: Some(Metadata {
                created_date: Utc::now().format("%Y-%m-%d").to_string(),
                last_modified: Utc::now().format("%Y-%m-%d").to_string(),
                version: "1.0".to_string(),
                status: "PENDING".to_string(),
                blockchain: BlockchainInfo {
                    network: "CBDC_TESTNET".to_string(),
                    deployment_pending: true,
                },
            }),
        };

        info!("✅ JSON structure built successfully");

        Ok(agreement)
    }
}