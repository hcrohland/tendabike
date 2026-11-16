#![warn(clippy::all)]

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    // The deprecation warnings for the legacy variables in use are printed
    // here, before the logging subscriber exists (it is set up by
    // `tb_axum::start`).
    let env = std::env::vars().collect();
    let (config, deprecations) = config::from_env(&env);
    for warning in &deprecations {
        eprintln!("warning: {warning}");
    }

    let pool = sqlx::PgPool::connect(&config.database_url).await?;

    // Build the provider config from the environment and initialize the
    // provider crate's client statics.
    let provider_config = tb_provider::ProviderConfig {
        strava_client_id: config.strava_client_id,
        strava_client_secret: config.strava_client_secret,
        strava_redirect_url: config.strava_redirect_url,
        icu_client_id: config.icu_client_id,
        icu_client_secret: config.icu_client_secret,
        icu_redirect_url: config.icu_redirect_url,
        icu_webhook_secret: config.icu_webhook_secret,
    };
    tb_provider::init(provider_config);

    // Build the web layer's config.
    let axum_config = tb_axum::AxumConfig {
        csrf_secret: std::env::var("CSRF_SECRET").ok(),
        static_www: std::env::var("STATIC_WWW").unwrap_or_else(|_| {
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../frontend/build").to_string()
        }),
        bind_addr: std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8000".to_string()),
    };

    // The composition root wires the concrete crates and process setup;
    // the crate-internal details (the session store, the logging
    // subscriber) live in `tb_axum::start`.
    let db_pool = tb_sqlx::DbPool::new(pool.clone()).await?;

    // Build the `Registry` over the pool (it owns the seam — the web layer
    // names the trait, never a seam type, spec #558 §1/§6.1) and hand the
    // web layer the service API handle. The raw pool handed to the tower
    // session store is unchanged (it is not a seam type). The `DetachedBegin`
    // wrapper provides the cancellation-safe `begin` (the spawn/oneshot
    // machinery that keeps a cancelled `BEGIN` from poisoning the pool) —
    // an executor-level concern, not a sqlx-wrapper concern.
    let registry = tb_exec::Registry::with_defaults(
        tb_exec::DetachedBegin::new(db_pool),
        tb_exec::DomainFacadeImpl,
        tb_provider::UnifiedLane::new(tb_provider::EntityFacadeImpl),
    );
    let api = std::sync::Arc::new(registry.clone()) as std::sync::Arc<dyn tb_exec::ServiceApi>;
    let provider = std::sync::Arc::new(registry)
        as std::sync::Arc<dyn tb_exec::providerservice::ProviderService>;

    Ok(tb_axum::start(api, provider, axum_config, pool.clone()).await?)
}
