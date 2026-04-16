use std::env;
use axum::http::HeaderValue;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub test_database_url: Option<String>,
    pub jwt_secret: String,
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub discord_redirect_url: String,
    pub frontend_url: String,
    pub frontend_origin: HeaderValue,
    pub server_host: String,
    pub server_port: u16,
}

impl Config {
    pub fn resolve_test_database_url_from_env() -> anyhow::Result<String> {
        if let Some(url) = env::var("TEST_DATABASE_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(url);
        }

        env::var("DATABASE_URL").map_err(|_| anyhow::anyhow!(
            "DATABASE_URL must be set (or provide TEST_DATABASE_URL for integration tests)"
        ))
    }

    pub fn effective_test_database_url(&self) -> &str {
        self.test_database_url
            .as_deref()
            .unwrap_or(&self.database_url)
    }

    pub fn from_env() -> anyhow::Result<Self> {
        let frontend_url = env::var("FRONTEND_URL")
            .unwrap_or_else(|_| "http://localhost:4200".to_string());

        let frontend_origin = frontend_url
            .parse::<HeaderValue>()
            .map_err(|err| anyhow::anyhow!("Invalid FRONTEND_URL for CORS allow_origin ('{frontend_url}'): {err}"))?;

        let server_port = env::var("SERVER_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .map_err(|err| anyhow::anyhow!("SERVER_PORT must be a valid u16: {err}"))?;

        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| anyhow::anyhow!("JWT_SECRET must be set"))?;

        if jwt_secret.trim().is_empty() {
            return Err(anyhow::anyhow!("JWT_SECRET must not be empty"));
        }

        if jwt_secret == "change-me-in-production" {
            return Err(anyhow::anyhow!(
                "JWT_SECRET is using an insecure placeholder value; set a strong random secret"
            ));
        }

        // Require a reasonably strong secret length to reduce weak-key misconfiguration.
        if jwt_secret.len() < 32 {
            return Err(anyhow::anyhow!("JWT_SECRET must be at least 32 characters long"));
        }

        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .map_err(|_| anyhow::anyhow!("DATABASE_URL must be set"))?,
            test_database_url: env::var("TEST_DATABASE_URL")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            jwt_secret,
            discord_client_id: env::var("DISCORD_CLIENT_ID").unwrap_or_default(),
            discord_client_secret: env::var("DISCORD_CLIENT_SECRET").unwrap_or_default(),
            discord_redirect_url: env::var("DISCORD_REDIRECT_URL").unwrap_or_else(|_| {
                "http://localhost:3000/api/auth/discord/callback".to_string()
            }),
            frontend_url,
            frontend_origin,
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            server_port,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Config;
    use std::{
        env,
        sync::{Mutex, MutexGuard, OnceLock},
    };

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn env_lock() -> &'static Mutex<()> {
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    fn acquire_env_lock() -> MutexGuard<'static, ()> {
        env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    const CONFIG_KEYS: [&str; 9] = [
        "DATABASE_URL",
        "TEST_DATABASE_URL",
        "JWT_SECRET",
        "FRONTEND_URL",
        "SERVER_PORT",
        "DISCORD_CLIENT_ID",
        "DISCORD_CLIENT_SECRET",
        "DISCORD_REDIRECT_URL",
        "SERVER_HOST",
    ];

    struct EnvGuard {
        saved: Vec<(&'static str, Option<String>)>,
    }

    impl EnvGuard {
        fn capture(keys: &[&'static str]) -> Self {
            let saved = keys
                .iter()
                .map(|key| (*key, env::var(key).ok()))
                .collect::<Vec<_>>();
            Self { saved }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, value) in &self.saved {
                if let Some(v) = value {
                    env::set_var(key, v);
                } else {
                    env::remove_var(key);
                }
            }
        }
    }

    fn set_required_env(jwt_secret: &str) {
        env::set_var("DATABASE_URL", "postgres://postgres:postgres@localhost/stars_reborn_test");
        env::set_var("JWT_SECRET", jwt_secret);
    }

    #[test]
    fn from_env_fails_when_database_url_is_missing() {
        let _lock = acquire_env_lock();
        let _guard = EnvGuard::capture(&CONFIG_KEYS);

        env::remove_var("DATABASE_URL");
        env::set_var("JWT_SECRET", "0123456789abcdef0123456789abcdef");

        let err = Config::from_env().expect_err("missing DATABASE_URL should fail");
        assert!(err.to_string().contains("DATABASE_URL must be set"));
    }

    #[test]
    fn from_env_fails_when_jwt_secret_is_missing() {
        let _lock = acquire_env_lock();
        let _guard = EnvGuard::capture(&CONFIG_KEYS);

        env::set_var("DATABASE_URL", "postgres://postgres:postgres@localhost/stars_reborn_test");
        env::remove_var("JWT_SECRET");

        let err = Config::from_env().expect_err("missing JWT_SECRET should fail");
        assert!(err.to_string().contains("JWT_SECRET must be set"));
    }

    #[test]
    fn from_env_fails_when_jwt_secret_is_empty_or_whitespace() {
        let _lock = acquire_env_lock();
        let _guard = EnvGuard::capture(&CONFIG_KEYS);

        set_required_env("   ");

        let err = Config::from_env().expect_err("whitespace JWT_SECRET should fail");
        assert!(err.to_string().contains("JWT_SECRET must not be empty"));
    }

    #[test]
    fn from_env_fails_when_jwt_secret_is_too_short() {
        let _lock = acquire_env_lock();
        let _guard = EnvGuard::capture(&CONFIG_KEYS);

        set_required_env("too-short");

        let err = Config::from_env().expect_err("short JWT_SECRET should fail");
        assert!(
            err.to_string()
                .contains("JWT_SECRET must be at least 32 characters long")
        );
    }

    #[test]
    fn from_env_fails_when_jwt_secret_is_known_placeholder() {
        let _lock = acquire_env_lock();
        let _guard = EnvGuard::capture(&CONFIG_KEYS);

        set_required_env("change-me-in-production");

        let err = Config::from_env().expect_err("placeholder JWT_SECRET should fail");
        assert!(
            err.to_string().contains("JWT_SECRET is using an insecure placeholder value")
        );
    }

    #[test]
    fn from_env_fails_when_server_port_is_invalid() {
        let _lock = acquire_env_lock();
        let _guard = EnvGuard::capture(&CONFIG_KEYS);

        set_required_env("0123456789abcdef0123456789abcdef");
        env::set_var("SERVER_PORT", "not-a-number");

        let err = Config::from_env().expect_err("invalid SERVER_PORT should fail");
        assert!(err.to_string().contains("SERVER_PORT must be a valid u16"));
    }

    #[test]
    fn from_env_fails_when_frontend_url_is_not_a_valid_header_value() {
        let _lock = acquire_env_lock();
        let _guard = EnvGuard::capture(&CONFIG_KEYS);

        set_required_env("0123456789abcdef0123456789abcdef");
        env::set_var("FRONTEND_URL", "http://localhost:4200\nmalicious");

        let err = Config::from_env().expect_err("invalid FRONTEND_URL should fail");
        assert!(
            err.to_string()
                .contains("Invalid FRONTEND_URL for CORS allow_origin")
        );
    }

    #[test]
    fn from_env_succeeds_with_strong_jwt_secret() {
        let _lock = acquire_env_lock();
        let _guard = EnvGuard::capture(&CONFIG_KEYS);

        set_required_env("0123456789abcdef0123456789abcdef");

        let cfg = Config::from_env().expect("valid config should parse");
        assert_eq!(cfg.jwt_secret, "0123456789abcdef0123456789abcdef");
        assert_eq!(cfg.server_port, 3000);
        assert_eq!(cfg.frontend_url, "http://localhost:4200");
        assert_eq!(
            cfg.effective_test_database_url(),
            "postgres://postgres:postgres@localhost/stars_reborn_test"
        );
    }

    #[test]
    fn effective_test_database_url_uses_override_when_present() {
        let _lock = acquire_env_lock();
        let _guard = EnvGuard::capture(&CONFIG_KEYS);

        set_required_env("0123456789abcdef0123456789abcdef");
        env::set_var(
            "TEST_DATABASE_URL",
            "postgres://postgres:postgres@localhost/stars_reborn_test_override",
        );

        let cfg = Config::from_env().expect("valid config should parse");
        assert_eq!(
            cfg.effective_test_database_url(),
            "postgres://postgres:postgres@localhost/stars_reborn_test_override"
        );
    }

    #[test]
    fn effective_test_database_url_falls_back_when_override_is_blank() {
        let _lock = acquire_env_lock();
        let _guard = EnvGuard::capture(&CONFIG_KEYS);

        set_required_env("0123456789abcdef0123456789abcdef");
        env::set_var("TEST_DATABASE_URL", "   ");

        let cfg = Config::from_env().expect("valid config should parse");
        assert_eq!(
            cfg.effective_test_database_url(),
            "postgres://postgres:postgres@localhost/stars_reborn_test"
        );
    }

    #[test]
    fn resolve_test_database_url_from_env_prefers_test_database_url() {
        let _lock = acquire_env_lock();
        let _guard = EnvGuard::capture(&CONFIG_KEYS);

        env::set_var("DATABASE_URL", "postgres://postgres:postgres@localhost/app_db");
        env::set_var(
            "TEST_DATABASE_URL",
            "postgres://postgres:postgres@localhost/test_db",
        );

        let resolved = Config::resolve_test_database_url_from_env()
            .expect("test database URL should resolve");
        assert_eq!(resolved, "postgres://postgres:postgres@localhost/test_db");
    }

    #[test]
    fn resolve_test_database_url_from_env_falls_back_to_database_url() {
        let _lock = acquire_env_lock();
        let _guard = EnvGuard::capture(&CONFIG_KEYS);

        env::set_var("DATABASE_URL", "postgres://postgres:postgres@localhost/app_db");
        env::set_var("TEST_DATABASE_URL", "   ");

        let resolved = Config::resolve_test_database_url_from_env()
            .expect("database URL fallback should resolve");
        assert_eq!(resolved, "postgres://postgres:postgres@localhost/app_db");
    }
}
