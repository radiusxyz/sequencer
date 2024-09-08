use crate::{client::liveness::seeder::SeederClient, types::Config};

pub struct AppState {
    config: Config,
    seeder_client: SeederClient,
}

impl AppState {
    pub fn new(config: Config, seeder_client: SeederClient) -> Self {
        Self {
            config,
            seeder_client,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn seeder_client(&self) -> &SeederClient {
        &self.seeder_client
    }

    pub fn is_using_zkp(&self) -> bool {
        true
    }
}
