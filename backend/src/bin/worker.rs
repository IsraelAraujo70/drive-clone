#[tokio::main]
async fn main() {
    drive_clone_api::bootstrap::worker::run().await;
}
