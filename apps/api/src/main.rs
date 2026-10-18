use knowtosay::{db::AppState, http, sync};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut root =
        std::env::var("KNOWTOSAY_DATA_DIR").unwrap_or_else(|_| ".local/development".into());
    let mut port = 4174u16;
    let mut background = true;
    let args: Vec<_> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--data-dir" => {
                i += 1;
                root = args.get(i).ok_or("missing data directory")?.clone()
            }
            "--port" => {
                i += 1;
                port = args.get(i).ok_or("missing port")?.parse()?
            }
            "--no-lan" => background = false,
            _ => return Err(format!("Unknown option: {}", args[i]).into()),
        }
        i += 1;
    }
    let root = std::path::absolute(root)?;
    let state = AppState::open(root).await.map_err(|e| e.2)?;
    if background {
        sync::background(state.clone())
    }
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    eprintln!("KnowToSay local API listening on 127.0.0.1:{port}");
    axum::serve(listener, http::local_router(state)).await?;
    Ok(())
}
