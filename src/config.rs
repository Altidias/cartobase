use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub admin_token: Option<String>,
    pub admin_crew: String,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL is required".to_string())?;
        let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
        let admin_token = env::var("CARTOBASE_ADMIN_TOKEN")
            .ok()
            .filter(|v| !v.trim().is_empty());
        let admin_crew = env::var("CARTOBASE_ADMIN_CREW").unwrap_or_else(|_| "default".into());
        Ok(Self { database_url, bind_addr, admin_token, admin_crew })
    }
}
