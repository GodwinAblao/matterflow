use std::{env, error::Error, fmt};

pub struct Config {
    pub host: String,
    pub port: u16,
    pub frontend_origin: String,
    pub database_url: Option<String>,
}

// Never store the original value or parser error here: both may contain secrets.
pub struct InvalidDatabaseUrl;
impl fmt::Display for InvalidDatabaseUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Invalid DATABASE_URL. In Render, set the key DATABASE_URL and use a single PostgreSQL URL starting with postgresql:// or postgres:// as its value. Do not paste a psql command, placeholder, or a variable reference. Credentials have not been displayed.")
    }
}
impl fmt::Debug for InvalidDatabaseUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}
impl Error for InvalidDatabaseUrl {}

fn normalize_database_url(input: Option<String>) -> Result<Option<String>, InvalidDatabaseUrl> {
    let Some(input) = input else {
        return Ok(None);
    };
    let mut value = input.trim();
    // Dashboard values are literal strings, unlike values loaded by dotenv.
    // Tolerate copying a complete .env assignment or one pair of outer quotes.
    if let Some(assignment) = value.strip_prefix("DATABASE_URL") {
        value = assignment
            .trim_start()
            .strip_prefix('=')
            .ok_or(InvalidDatabaseUrl)?
            .trim();
    }
    if let Some(unquoted) = value
        .strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
    {
        value = unquoted.trim();
    } else if let Some(unquoted) = value
        .strip_prefix('\'')
        .and_then(|text| text.strip_suffix('\''))
    {
        value = unquoted.trim();
    }
    if value.is_empty() {
        return Ok(None);
    }
    if value.contains('\n')
        || value.contains('\r')
        || !(value.starts_with("postgresql://") || value.starts_with("postgres://"))
    {
        return Err(InvalidDatabaseUrl);
    }
    value
        .parse::<sqlx::postgres::PgConnectOptions>()
        .map_err(|_| InvalidDatabaseUrl)?;
    Ok(Some(value.to_owned()))
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into()),
            port: env::var("PORT").unwrap_or_else(|_| "3000".into()).parse()?,
            frontend_origin: env::var("FRONTEND_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:5173".into()),
            database_url: normalize_database_url(env::var("DATABASE_URL").ok())?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_database_url;
    const URL: &str = "postgresql://demo:not-a-real-password@db.example.test/matterflow?sslmode=require&channel_binding=require";

    #[test]
    fn preserves_raw_postgresql_url_and_ssl_options() {
        assert_eq!(
            normalize_database_url(Some(URL.into())).unwrap().as_deref(),
            Some(URL)
        );
    }
    #[test]
    fn accepts_dashboard_quotes_and_copied_env_assignment() {
        for value in [
            format!("  \"{URL}\"  "),
            format!("'{URL}'"),
            format!("DATABASE_URL=\"{URL}\""),
            format!("DATABASE_URL = '{URL}'"),
        ] {
            assert_eq!(
                normalize_database_url(Some(value)).unwrap().as_deref(),
                Some(URL)
            );
        }
    }
    #[test]
    fn accepts_postgres_scheme() {
        let value = URL.replacen("postgresql://", "postgres://", 1);
        assert_eq!(
            normalize_database_url(Some(value.clone())).unwrap(),
            Some(value)
        );
    }
    #[test]
    fn leaves_database_optional_for_local_framework_startup() {
        for value in [None, Some(String::new()), Some("  ".into())] {
            assert!(normalize_database_url(value).unwrap().is_none());
        }
    }
    #[test]
    fn rejects_non_urls_and_commands_without_echoing_credentials() {
        for value in [
            "$DATABASE_URL",
            "neondb_owner:not-a-real-password@host/db",
            "psql postgresql://demo:not-a-real-password@host/db",
            "https://db.example.test/matterflow",
            "postgresql://demo:not-a-real-password@host:invalid/db",
        ] {
            let error = normalize_database_url(Some(value.into())).unwrap_err();
            assert!(error.to_string().contains("Invalid DATABASE_URL"));
            assert!(!format!("{error:?}").contains("not-a-real-password"));
        }
    }
    #[test]
    fn rejects_multiline_values() {
        assert!(normalize_database_url(Some(format!("{URL}\nHOST=0.0.0.0"))).is_err());
    }
}
