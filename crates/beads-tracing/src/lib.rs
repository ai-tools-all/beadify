use tracing_subscriber::{EnvFilter, FmtSubscriber};

pub fn init() {
    if tracing::subscriber::set_global_default(
        FmtSubscriber::builder()
            .with_env_filter(EnvFilter::from_default_env())
            .with_writer(std::io::stderr)
            .finish(),
    )
    .is_err()
    {
        // Subscriber was already set; ignore.
    }
}
