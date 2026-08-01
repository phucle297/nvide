fn main() {
    if let Err(error) = run() {
        eprintln!("nvide: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "xtask")]
    if std::env::args()
        .nth(1)
        .as_deref()
        .is_some_and(|arg| arg == "schema" || arg == "evidence")
    {
        return nvide_rpc_schema::xtask::run(std::env::args().skip(1));
    }

    nvide_ui::run().map_err(Into::into)
}
