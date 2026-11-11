//! Display-only native dialog labels. Never controls filters or file grants.
use crate::dto::AppErrorDto;
use serde::Deserialize;

/// Stable UI classification alongside the original technical explanation.
/// This does not parse English strings or change the underlying capability gate.
pub fn serialize_read_only<S: serde::Serializer>(
    reason: &Option<String>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    #[derive(serde::Serialize)]
    struct Reason<'a> {
        code: &'static str,
        details: &'a str,
    }
    serde::Serialize::serialize(
        &reason.as_ref().map(|details| Reason {
            code: "read-only",
            details,
        }),
        serializer,
    )
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DialogLabels {
    pub title: String,
    pub file_type: String,
    pub all_files: String,
}

impl DialogLabels {
    pub fn validate(&self) -> Result<(), AppErrorDto> {
        for label in [&self.title, &self.file_type, &self.all_files] {
            if label.is_empty() || label.len() > 512 || label.chars().any(char::is_control) {
                return Err(AppErrorDto {
                    cause: None,
                    code: "bad-request",
                    message: "invalid dialog label".into(),
                    params: Default::default(),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_keep_machine_parameters_and_causes_separate_from_details() {
        let too_large = AppErrorDto::file_too_large(1234);
        let json = serde_json::to_value(too_large).unwrap();
        assert_eq!(json["params"]["limit"], 1234);
        assert_eq!(json["code"], "file-too-large");
        let invalid = AppErrorDto::edit_failed(h3forge_save::EditError::InvalidValue {
            field: "army count",
            reason: "technical detail".into(),
        });
        assert_eq!(invalid.cause, Some("invalid-value"));
        assert!(invalid.message.contains("technical detail"));
        #[derive(serde::Serialize)]
        struct Example {
            #[serde(serialize_with = "serialize_read_only")]
            reason: Option<String>,
        }
        let json = serde_json::to_value(Example {
            reason: Some("unknown bytes".into()),
        })
        .unwrap();
        assert_eq!(json["reason"]["code"], "read-only");
        assert_eq!(json["reason"]["details"], "unknown bytes");
        assert!(serde_json::to_value(Example { reason: None }).unwrap()["reason"].is_null());
    }

    #[test]
    fn dialog_labels_are_bounded_unicode_text_only() {
        let mut labels = DialogLabels {
            title: "Открыть сохранение".into(),
            file_type: "Spielstände".into(),
            all_files: "Todos los archivos".into(),
        };
        assert!(labels.validate().is_ok());
        labels.title.push('\0');
        assert!(labels.validate().is_err());
        labels.title = "a".repeat(513);
        assert!(labels.validate().is_err());
        labels.title.clear();
        assert!(labels.validate().is_err());
    }
}
