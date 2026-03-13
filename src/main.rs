use ashpd::desktop::screenshot::Screenshot;

#[tokio::main]
async fn main() -> ashpd::Result<()> {
    let response = Screenshot::request()
        .interactive(true)
        .modal(true)
        .send()
        .await?;
    let screenshot = response.response()?;

    println!("Saved screenshot URI: {}", screenshot.uri());

    Ok(())
}
