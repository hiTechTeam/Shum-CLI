//! Contact addressing lives in Shum-Core; the CLI words its failures.
use anyhow::{anyhow, Result};
pub use shum_session::contact::network_id;

fn worded(failure: shum_session::Failure) -> anyhow::Error {
    anyhow!("{}", crate::runtime::describe(&failure))
}
pub fn validate(selector: &str) -> Result<()> {
    shum_session::contact::validate(selector).map_err(worded)
}
pub fn resolve<'a>(
    selector: &str,
    candidates: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Result<Option<String>> {
    shum_session::contact::resolve(selector, candidates).map_err(worded)
}
