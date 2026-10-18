use utoipa::OpenApi;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        knowtosay::contract::ApiDoc::openapi().to_pretty_json()?
    );
    Ok(())
}
