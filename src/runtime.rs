//! The running profile lives in Shum-Core (`shum-session`). This module keeps
//! the CLI's paths and words session failures in the interface language.
use crate::i18n::t;
pub use shum_session::{
    now, random, uuid, Command, Failure, Request, Session as Runtime, COMMAND_SCHEMA_VERSION,
};

/// A session failure as the CLI shows it.
pub fn describe(failure: &Failure) -> String {
    match failure {
        Failure::Text { text } => t(text).to_string(),
        Failure::Phase { phase } => crate::invitations::hint(phase).to_string(),
    }
}
