use std::borrow::Cow;

use chrono::Duration;
use cookie::{SameSite, Key};

#[derive(Clone,)] //Debug
pub struct AxumSessionConfig{
    pub(crate) idle_timeout:Duration,
    pub(crate) memory_clear_timeout:Duration, 

    /// Session cookie domain
    pub(crate) cookie_domain: Option<Cow<'static, str>>,
    /// Session cookie http only flag
    pub(crate) cookie_http_only: bool,
    /// Session cookie path
    pub(crate) cookie_path: Cow<'static, str>,
    /// Resticts how Cookies are sent cross-site. Default is `SameSite::Lax`
    pub(crate) cookie_same_site: SameSite,
    /// Session cookie secure flag
    pub(crate) cookie_secure: bool,

    ///Encyption Key used to encypt cookies for confidentiality, integrity, and authenticity.
    pub(crate) key: Option<Key>,
}

impl std::fmt::Debug for AxumSessionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AxumSessionConfig")
            .field("idle_timeout", &self.idle_timeout)
            .field("memory_clear_timeout", &self.memory_clear_timeout)
            .field("cookie_domain", &self.cookie_domain)
            .field("cookie_http_only", &self.cookie_http_only)
            .field("cookie_path", &self.cookie_path)
            .field("cookie_same_site", &self.cookie_same_site)
            .field("cookie_secure", &self.cookie_secure)
            .field("key", &"key hidden")
            .finish()
    }
}


impl Default for AxumSessionConfig{
    fn default() -> Self{
        AxumSessionConfig{
            idle_timeout:Duration::days(300),
            memory_clear_timeout:Duration::minutes(10),

            cookie_path: "/".into(),
            cookie_http_only: true,
            cookie_secure: false,
            cookie_domain: None,
            cookie_same_site: SameSite::Lax,

            key:None,
        }
    }
}