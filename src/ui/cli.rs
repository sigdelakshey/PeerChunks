use tokio::sync::mpsc::Sender;
use log::error;

pub async fn run_cli(tx: Sender<String>) {
    loop {
        println!("Enter command (upload/download/search/exit): ");
        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(_) => {
                let input = input.trim().to_string();
                if input.eq_ignore_ascii_case("exit") {
                    println!("Exiting PeerChunks CLI.");
                    break;
                }
                if let Err(e) = tx.send(input).await {
                    error!("Failed to send command to processor: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to read line: {}", e);
            }
        }
    }
}
