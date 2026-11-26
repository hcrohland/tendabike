#![warn(clippy::all)]
#![warn(unreachable_pub)]

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
    // The unified provider lane (ADR-0014): the registry drives a single
    // lane, which resolves the user's provider and dispatches to the right
    // driver internally — so an icu user's queued events are processed
    // through their own (marker, driver) lane, and the registry is
    // provider-ignorant.
    let registry = tb_exec::Registry::with_defaults(
        tb_exec::DetachedBegin::new(db_pool),
        tb_exec::DomainFacadeImpl,
        tb_provider::UnifiedLaneImpl::<_>::new(
            tb_provider::ProviderLaneImpl::<
                tb_provider::strava::StravaSpec,
                _,
                tb_provider::strava::driver::StravaDriver,
            >::new(tb_provider::EntityFacadeImpl),
            tb_provider::ProviderLaneImpl::<
                tb_provider::icu::IcuSpec,
                _,
                tb_provider::icu::driver::IcuDriver,
            >::new(tb_provider::EntityFacadeImpl),
        ),
    );
    let api = std::sync::Arc::new(registry.clone()) as std::sync::Arc<dyn tb_exec::ServiceApi>;
    let provider =
        std::sync::Arc::new(registry) as std::sync::Arc<dyn tb_provider::ProviderService>;

    Ok(tb_axum::start(api, provider, config.axum, pool.clone()).await?)
}
