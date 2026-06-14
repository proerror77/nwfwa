use serde_json::{json, Value};

pub(super) fn inbox_schemas() -> Value {
    json!({
                "InboxNormalizeRequest": {
                    "type": "object",
                    "description": "Customer-specific raw claim intake payload. MVP supports the AiClaim Core reportCase envelope.",
                    "required": ["systemCode", "transNo", "reportCase"],
                    "properties": {
                        "systemCode": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Source system code bound to the authenticated API key."
                        },
                        "transNo": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Source transaction id used with reportNo for idempotency."
                        },
                        "transDate": {
                            "type": ["string", "null"],
                            "description": "Source transaction timestamp when present."
                        },
                        "reportCase": {
                            "type": "object",
                            "description": "Raw source claim case payload. It may contain medical records, policy, invoice, product, and liability lists."
                        }
                    },
                    "additionalProperties": true
                },
                "InboxNormalizeResponse": {
                    "type": "object",
                    "required": [
                        "run_id",
                        "audit_id",
                        "mapping_version",
                        "raw_payload_checksum",
                        "validation_result",
                        "scoring_ready",
                        "validation_errors",
                        "canonical_claim_context",
                        "data_quality_signals",
                        "evidence_refs"
                    ],
                    "properties": {
                        "run_id": { "type": "string" },
                        "audit_id": { "type": "string" },
                        "external_message_id": { "type": ["string", "null"] },
                        "idempotency_key": { "type": ["string", "null"] },
                        "raw_payload_checksum": { "type": "string" },
                        "mapping_version": { "type": "string" },
                        "validation_result": {
                            "type": "string",
                            "enum": ["accepted", "accepted_with_warnings", "rejected"]
                        },
                        "scoring_ready": { "type": "boolean" },
                        "raw_payload_ref": { "type": ["string", "null"] },
                        "validation_errors": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/InboxValidationError" }
                        },
                        "canonical_claim_context": {
                            "$ref": "#/components/schemas/InboxCanonicalClaimContext"
                        },
                        "data_quality_signals": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "InboxCanonicalClaimContext": {
                    "type": "object",
                    "required": [
                        "claim_header",
                        "member_policy_snapshot",
                        "provider_snapshot",
                        "itemized_bill_lines",
                        "document_evidence"
                    ],
                    "properties": {
                        "claim_header": { "$ref": "#/components/schemas/InboxClaimHeader" },
                        "member_policy_snapshot": { "$ref": "#/components/schemas/InboxMemberPolicySnapshot" },
                        "provider_snapshot": { "$ref": "#/components/schemas/InboxProviderSnapshot" },
                        "itemized_bill_lines": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/InboxBillLine" }
                        },
                        "document_evidence": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/InboxDocumentEvidence" }
                        }
                    }
                },
                "InboxClaimHeader": {
                    "type": "object",
                    "properties": {
                        "external_claim_id": { "type": "string" },
                        "source_system": { "type": "string" },
                        "service_date": { "type": ["string", "null"], "format": "date" },
                        "receive_date": { "type": ["string", "null"], "format": "date" },
                        "accident_date": { "type": ["string", "null"], "format": "date" },
                        "source_timezone": { "type": "string" },
                        "service_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "receive_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "accident_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "accident_reason": { "type": ["string", "null"] },
                        "medical_type": { "type": ["string", "null"] },
                        "currency": { "type": "string" },
                        "total_amount": { "type": ["number", "null"] }
                    }
                },
                "InboxMemberPolicySnapshot": {
                    "type": "object",
                    "properties": {
                        "masked_member_id": { "type": ["string", "null"] },
                        "masked_certificate_id": { "type": ["string", "null"] },
                        "certificate_type": { "type": ["string", "null"] },
                        "member_gender": { "type": ["string", "null"] },
                        "member_birth_date": { "type": ["string", "null"], "format": "date" },
                        "source_timezone": { "type": "string" },
                        "member_birth_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "policy_id": { "type": ["string", "null"] },
                        "product_code": { "type": ["string", "null"] },
                        "liability_code": { "type": ["string", "null"] },
                        "liability_name": { "type": ["string", "null"] },
                        "policy_type": { "type": ["string", "null"] },
                        "policy_first_apply_date": { "type": ["string", "null"], "format": "date" },
                        "policy_first_apply_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "insured_with_social_insurance": { "type": ["boolean", "null"] },
                        "coverage_limit": { "type": ["number", "null"] },
                        "coverage_start_date": { "type": ["string", "null"], "format": "date" },
                        "coverage_end_date": { "type": ["string", "null"], "format": "date" },
                        "coverage_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "coverage_end_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "liability_start_date": { "type": ["string", "null"], "format": "date" },
                        "liability_claim_start_date": { "type": ["string", "null"], "format": "date" },
                        "waiting_period_end_date": { "type": ["string", "null"], "format": "date" },
                        "liability_end_date": { "type": ["string", "null"], "format": "date" },
                        "liability_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "liability_claim_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "liability_end_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "product_liabilities": {
                            "type": "array",
                            "description": "All product and claim-liability windows from the source policies, preserving coverage and waiting-period candidates before scoring.",
                            "items": { "$ref": "#/components/schemas/InboxProductLiability" }
                        }
                    }
                },
                "InboxProductLiability": {
                    "type": "object",
                    "properties": {
                        "policy_id": { "type": ["string", "null"] },
                        "product_id": { "type": ["string", "null"] },
                        "product_code": { "type": ["string", "null"] },
                        "product_name": { "type": ["string", "null"] },
                        "plan_code": { "type": ["string", "null"] },
                        "plan_version": { "type": ["string", "null"] },
                        "product_start_date": { "type": ["string", "null"], "format": "date" },
                        "product_end_date": { "type": ["string", "null"], "format": "date" },
                        "source_timezone": { "type": "string" },
                        "product_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "product_end_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "liability_id": { "type": ["string", "null"] },
                        "liability_code": { "type": ["string", "null"] },
                        "liability_name": { "type": ["string", "null"] },
                        "liability_start_date": { "type": ["string", "null"], "format": "date" },
                        "liability_claim_start_date": { "type": ["string", "null"], "format": "date" },
                        "waiting_period_end_date": { "type": ["string", "null"], "format": "date" },
                        "liability_end_date": { "type": ["string", "null"], "format": "date" },
                        "liability_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "liability_claim_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "liability_end_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "is_serious_disease_liability": { "type": ["boolean", "null"] },
                        "main_liability": { "type": ["boolean", "null"] },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "InboxProviderSnapshot": {
                    "type": "object",
                    "properties": {
                        "provider_code": { "type": ["string", "null"] },
                        "name": { "type": ["string", "null"] },
                        "class": { "type": ["string", "null"] },
                        "type": { "type": ["string", "null"] },
                        "city": { "type": ["string", "null"] },
                        "province": { "type": ["string", "null"] },
                        "network_flags": { "$ref": "#/components/schemas/InboxProviderNetworkFlags" }
                    }
                },
                "InboxProviderNetworkFlags": {
                    "type": "object",
                    "properties": {
                        "is_hospital_institution": { "type": ["boolean", "null"] },
                        "primary_care": { "type": ["boolean", "null"] },
                        "red_flag": { "type": ["string", "null"] }
                    }
                },
                "InboxBillLine": {
                    "type": "object",
                    "properties": {
                        "invoice_id": { "type": ["string", "null"] },
                        "diagnosis_list": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/InboxDiagnosis" }
                        },
                        "fee_category": { "type": ["string", "null"] },
                        "item_name": { "type": ["string", "null"] },
                        "amount": { "type": ["number", "null"] },
                        "self_pay": { "type": ["number", "null"] },
                        "own_expense": { "type": ["number", "null"] },
                        "social_insurance_amount": { "type": ["number", "null"] },
                        "medical_category": { "type": ["string", "null"] },
                        "invoice_bill_type": { "type": ["string", "null"] },
                        "invoice_document_type": { "type": ["string", "null"] },
                        "social_insurance_type": { "type": ["string", "null"] },
                        "department": { "type": ["string", "null"] },
                        "medical_type": { "type": ["string", "null"] },
                        "invoice_claim_nature": { "type": ["string", "null"] },
                        "invoice_start_date": { "type": ["string", "null"], "format": "date" },
                        "invoice_end_date": { "type": ["string", "null"], "format": "date" },
                        "source_timezone": { "type": "string" },
                        "invoice_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "invoice_end_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "invoice_social_insurance_amount": { "type": ["number", "null"] },
                        "invoice_self_pay_amount": { "type": ["number", "null"] },
                        "invoice_own_expense_amount": { "type": ["number", "null"] },
                        "invoice_other_amount": { "type": ["number", "null"] },
                        "invoice_provider_code": { "type": ["string", "null"] },
                        "invoice_provider_name": { "type": ["string", "null"] },
                        "invoice_provider_class": { "type": ["string", "null"] },
                        "invoice_provider_type": { "type": ["string", "null"] },
                        "invoice_provider_city": { "type": ["string", "null"] },
                        "invoice_provider_province": { "type": ["string", "null"] },
                        "invoice_is_hospital_institution": { "type": ["boolean", "null"] },
                        "invoice_primary_care": { "type": ["boolean", "null"] },
                        "invoice_red_flag": { "type": ["string", "null"] },
                        "fee_group_amount": { "type": ["number", "null"] },
                        "fee_group_other_amount": { "type": ["number", "null"] },
                        "medicare_prorated": { "type": ["string", "null"] },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "InboxDiagnosis": {
                    "type": "object",
                    "properties": {
                        "code": { "type": ["string", "null"] },
                        "name": { "type": ["string", "null"] }
                    }
                },
                "InboxDocumentEvidence": {
                    "type": "object",
                    "properties": {
                        "document_id": { "type": ["string", "null"] },
                        "department": { "type": ["string", "null"] },
                        "diagnosis": { "type": ["string", "null"] },
                        "claim_nature": { "type": ["string", "null"] },
                        "medical_record_type": { "type": ["string", "null"] },
                        "chief_complaint": { "type": ["string", "null"] },
                        "current_medical_history": { "type": ["string", "null"] },
                        "past_history": { "type": ["string", "null"] },
                        "extracted_diagnosis": { "type": ["string", "null"] },
                        "extracted_procedure": { "type": ["string", "null"] },
                        "extracted_prescription": { "type": ["string", "null"] },
                        "medical_type": { "type": ["string", "null"] },
                        "visit_date": { "type": ["string", "null"], "format": "date" },
                        "first_happen_date": { "type": ["string", "null"], "format": "date" },
                        "operation_start_date": { "type": ["string", "null"], "format": "date" },
                        "source_timezone": { "type": "string" },
                        "visit_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "first_happen_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "operation_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "medical_record_text": { "type": ["string", "null"] },
                        "source_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "InboxValidationError": {
                    "type": "object",
                    "required": ["field_path", "severity", "remediation"],
                    "properties": {
                        "field_path": { "type": "string" },
                        "severity": { "type": "string", "enum": ["error", "warning"] },
                        "remediation": { "type": "string" }
                    }
                },
    })
}
