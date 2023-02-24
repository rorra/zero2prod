use secrecy::Secret;
use secrecy::ExposeSecret;
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::ConnectOptions;
use sqlx::postgres::{PgConnectOptions, PgSslMode};

#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
}

#[derive(serde::Deserialize)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}

pub enum Environment {
    Development,
    Production,
    Test,
}

impl Environment {
    pub fn as_str(&self) -> &str {
        match self {
            Environment::Development => "development",
            Environment::Production => "production",
            Environment::Test => "test",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "development" => Ok(Self::Development),
            "production" => Ok(Self::Production),
            "test" => Ok(Self::Test),
            _ => Err(format!("Unknown environment: {}", value)),
        }
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let mut settings = config::Config::default();
    let base_path = std::env::current_dir().expect("Failed to determine the current directory.");
    let configuration_directory = base_path.join("config");

    settings.merge(config::File::from(configuration_directory.join("base")).required(true))?;

    // Running environment
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "development".into())
        .try_into()
        .expect("Invalid environment.");

    settings.merge(
        config::File::from(configuration_directory.join(environment.as_str())).required(true),
    )?;

    // Add settings app environment variables, like APP_APPLICATION__PORT=5001
    settings.merge(config::Environment::with_prefix("app").separator("__"))?;

    settings.try_into()
}

impl DatabaseSettings {
    pub fn without_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(&self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
    }

    pub fn with_db(&self) -> PgConnectOptions {
        let mut options = self.without_db().database(&self.database_name);
        options.log_statements(tracing::log::LevelFilter::Trace);
        options
    }
}