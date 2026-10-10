//! Invitation timeline data lives in Shum-Core; the CLI words its events.
use crate::i18n::t;
use serde_json::Value;
pub use shum_session::timeline::{is_invitation, project, stored, timeline};

pub fn label(event: &Value) -> &str {
    match event["kind"].as_str().unwrap_or("") {
        "invitationSent" => t("Приглашение отправлено. Дождитесь ответа."),
        "invitationReceived" => t("Получено приглашение в чат."),
        "invitationAccepted" => t("Собеседник принял приглашение. Можно общаться."),
        "invitationAcceptedLocally" => t("Вы приняли приглашение. Можно общаться."),
        "invitationDeclined" => t("Собеседник отклонил приглашение."),
        "invitationDeclinedLocally" => t("Вы отклонили приглашение."),
        _ => t("Чат доступен."),
    }
}

pub fn hint(phase: &str) -> &str {
    match phase {
        "incomingPending" => t("/accept принять · /decline отклонить"),
        "outgoingPending" => t("Приглашение отправлено. Дождитесь ответа."),
        "declinedLocally" => t("/accept принять, если передумали"),
        "declinedByPeer" => t("Приглашение отклонено. Собеседник может принять его позже."),
        "accepted" => t("Чат доступен."),
        _ => t("/invite пригласить"),
    }
}
