#![warn(clippy::all)]

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://localhost/tendabike".to_string());

    let pool = sqlx::PgPool::connect(&database_url).await?;

    // Build the provider config from the environment and initialize the
    // provider crate's client statics.
    let provider_config = tb_provider::ProviderConfig {
        strava_client_id: std::env::var("CLIENT_ID").expect("Missing CLIENT_ID!"),
        strava_client_secret: std::env::var("CLIENT_SECRET").expect("Missing CLIENT_SECRET!"),
        strava_redirect_url: std::env::var("REDIRECT_URL")
            .unwrap_or_else(|_| "http://localhost:8000/strava/token".to_string()),
        icu_client_id: std::env::var("ICU_CLIENT_ID").expect("Missing ICU_CLIENT_ID!"),
        icu_client_secret: std::env::var("ICU_CLIENT_SECRET").expect("Missing ICU_CLIENT_SECRET!"),
        icu_redirect_url: std::env::var("ICU_REDIRECT_URL")
            .unwrap_or_else(|_| "http://localhost:8000/icu/token".to_string()),
        icu_webhook_secret: std::env::var("ICU_WEBHOOK_SECRET").unwrap_or_default(),
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
