#[tokio::main]
async fn main() {
    drive_clone_api::bootstrap::server::run().await;
}
