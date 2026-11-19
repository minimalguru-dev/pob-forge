//! What identifies this app to the services it talks to. Addresses and client
//! registrations come from the build environment, so a build without them
//! leaves those features off instead of calling someone else's servers.

/// The CDN that serves tree art, item art and game data (`infra/cdn`), set at
/// build time with `POB_FORGE_CDN_URL`. `None` when the build has none.
pub(crate) fn cdn_url() -> Option<&'static str> {
    option_env!("POB_FORGE_CDN_URL").map(|u| u.trim_end_matches('/')).filter(|u| !u.is_empty())
}

/// The pathofexile.com OAuth client registered for this app, set at build time
/// with `POB_FORGE_GGG_CLIENT_ID`. Account sign-in is off without one.
pub(crate) fn ggg_client_id() -> Option<&'static str> {
    option_env!("POB_FORGE_GGG_CLIENT_ID").filter(|id| !id.is_empty())
}

/// GGG's API rules want a contact in every OAuth client's user agent.
pub(crate) fn ggg_contact() -> &'static str {
    option_env!("POB_FORGE_CONTACT").filter(|c| !c.is_empty()).unwrap_or("https://github.com/minimalguru-dev/pob-forge/issues")
}

pub(crate) const NO_CDN: &str = "This build has no download server configured (POB_FORGE_CDN_URL at build time).";
