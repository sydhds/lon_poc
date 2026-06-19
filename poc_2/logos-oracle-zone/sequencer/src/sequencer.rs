use std::fs;
use std::path::Path;
use std::time::Duration;
use anyhow::anyhow;
use rand::Rng;
use tokio::sync::mpsc::UnboundedSender;
use url::Url;
use logos_blockchain_zone_sdk::{
    adapter::NodeHttpClient,
    sequencer::{Event, SequencerCheckpoint, SequencerHandle, ZoneSequencer},
};
use lb_common_http_client::{BasicAuthCredentials, CommonHttpClient};
use lb_core::mantle::ops::channel::ChannelId;
use lb_key_management_system_service::keys::{ED25519_SECRET_KEY_SIZE, Ed25519Key};

pub struct Sequencer {
    sequencer: ZoneSequencer<NodeHttpClient>,
    handle: SequencerHandle<NodeHttpClient>,
    // state: InMemoryZoneState,
    // pub queue_file: String,
    // pub checkpoint_path: String,
}

impl Sequencer {

    pub(crate) fn new(
        node_endpoint: &str,
        signing_key_path: &str,
        node_auth_username: Option<String>,
        node_auth_password: Option<String>,
        queue_file: &str,
        checkpoint_path: &str,
        channel_path: &str
    ) -> anyhow::Result<Self> {

        let checkpoint = None;

        let signing_key = load_or_create_signing_key(Path::new(signing_key_path));
        let channel_id = ChannelId::from(signing_key.public_key().to_bytes());
        let node_url = Url::parse(node_endpoint)?; // .map_err(|e| anyhow!(e))?;
        let basic_auth = node_auth_username
            .map(|username| BasicAuthCredentials::new(username, node_auth_password));

        let node = NodeHttpClient::new(CommonHttpClient::new(basic_auth), node_url);
        let (sequencer, handle) = ZoneSequencer::init(channel_id, signing_key, node, checkpoint);

        Ok(Self {
            sequencer,
            handle,
            // state: InMemoryZoneState::default(),
            // queue_file: queue_file.to_owned(),
            // checkpoint_path: checkpoint_path.to_owned(),
        })
    }

    pub async fn run(&mut self) {

        // let Self { mut sequencer, handle, mut state, queue_file, checkpoint_path } = self;

        let mut sequencer_handle = self.handle.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            loop {
                interval.tick().await;
                sequencer_handle.wait_ready().await;
                /*
                if let Err(e) = process_pending_batch(&queue_file, &batch_handle).await {
                    error!("Batch processing failed: {e}");
                }
                */
                // TODO: call ask_and_write_price
                /*
                if let Err(e) = ask_and_write_price(&sequencer_handle).await {
                    eprintln!("Error while writing price update: {}", e);
                }
                */
            }
        });

        loop {
            let Some(event) = self.sequencer.next_event().await else { continue; };
            // handle_event(event, &handle, &mut state, &checkpoint_path).await;
            println!("Should handle event: {:?}", event);
        }
    }

}

fn load_or_create_signing_key(path: &Path) -> Ed25519Key {
    if path.exists() {
        let key_bytes = fs::read(path).expect("failed to read key file");
        assert!(
            key_bytes.len() == ED25519_SECRET_KEY_SIZE,
            "invalid key file: expected {} bytes, got {}",
            ED25519_SECRET_KEY_SIZE,
            key_bytes.len()
        );
        let key_array: [u8; ED25519_SECRET_KEY_SIZE] =
            key_bytes.try_into().expect("length already checked");
        Ed25519Key::from_bytes(&key_array)
    } else {
        let mut key_bytes = [0u8; ED25519_SECRET_KEY_SIZE];
        let mut rng = rand::thread_rng();
        rng.fill(&mut key_bytes);
        fs::write(path, key_bytes).expect("failed to write key file");
        Ed25519Key::from_bytes(&key_bytes)
    }
}

enum PriceUpdate {
    Request,
    Respond(u64),
}

async fn ask_and_write_price(queue: UnboundedSender<tokio::sync::oneshot::Sender<PriceUpdate>>, handle: &SequencerHandle<NodeHttpClient>) -> anyhow::Result<u64> {

    // TODO: use rocksdb to read the price

    /*
    let (one_tx, one_rx) = tokio::sync::oneshot::channel();
    queue.send(one_tx)?;
    let respond = one_rx.await?;
    match respond {
        PriceUpdate::Request => panic!(),
        PriceUpdate::Respond(price) => { Ok(price) }
    }
    */

    Ok(42)
}