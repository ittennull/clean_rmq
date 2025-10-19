use clean_rmq::Args;
use rabbitmq_http_client::blocking_api::Client;
use rabbitmq_http_client::commons::{ExchangeType, QueueType};
use rabbitmq_http_client::requests::{
    ExchangeParams, MessageProperties, QueueParams, VirtualHostParams,
};
use rand::distr::Alphanumeric;
use rand::Rng;
use std::error::Error;
use std::time::Duration;

pub struct TestClient<'a> {
    client: Client<&'a str, &'a str, &'a str>,
    vhost: String,
}

impl<'a> TestClient<'a> {
    pub fn new() -> Result<TestClient<'a>, Box<dyn Error>> {
        let vhost: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();

        let client = Client::new("http://localhost:15672/api", "guest", "guest");
        client.create_vhost(&VirtualHostParams {
            name: &vhost,
            description: None,
            tags: None,
            default_queue_type: None,
            tracing: false,
        })?;

        Ok(TestClient { client, vhost })
    }

    pub fn create_exchange(&self, name: &str) -> Result<(), Box<dyn Error>> {
        self.client.declare_exchange(
            &self.vhost,
            &ExchangeParams {
                name,
                exchange_type: ExchangeType::Fanout,
                durable: false,
                auto_delete: false,
                arguments: None,
            },
        )?;
        Ok(())
    }

    pub fn create_queue(&self, name: &str) -> Result<(), Box<dyn Error>> {
        self.client.declare_queue(
            &self.vhost,
            &QueueParams {
                name,
                queue_type: QueueType::Classic,
                durable: false,
                auto_delete: false,
                exclusive: false,
                arguments: None,
            },
        )?;
        Ok(())
    }

    pub fn create_connected_queue(&self, name: &str, exchange: &str) -> Result<(), Box<dyn Error>> {
        self.create_queue(name)?;
        self.client
            .bind_queue(&self.vhost, name, exchange, None, None)?;
        Ok(())
    }

    pub fn bind_exchange(&self, from: &str, to: &str) -> Result<(), Box<dyn Error>> {
        self.client
            .bind_exchange(&self.vhost, to, from, None, None)?;
        Ok(())
    }

    pub fn publish_message_and_wait_delivery_in(
        &self,
        exchange: &str,
        queue: &str,
    ) -> Result<(), Box<dyn Error>> {
        let messages_before = self.get_number_of_messages(queue)?;

        self.client
            .publish_message(&self.vhost, exchange, "", "data", MessageProperties::new())?;

        // wait for the message to arrive
        for _ in 0..10 {
            std::thread::sleep(Duration::from_secs(1));
            if messages_before < self.get_number_of_messages(queue)? {
                return Ok(());
            }
        }

        panic!(
            "No message delivered to queue '{}' via exchange '{}'",
            queue, exchange
        );
    }

    pub fn get_number_of_messages(&self, queue: &str) -> Result<u64, Box<dyn Error>> {
        let info = self.client.get_queue_info(&self.vhost, queue)?;
        Ok(info.message_count)
    }

    pub fn list_exchanges(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let exchanges = self.client.list_exchanges_in(&self.vhost)?;
        Ok(exchanges.into_iter().map(|x| x.name).collect())
    }

    pub fn list_queues(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let queues = self.client.list_queues_in(&self.vhost)?;
        Ok(queues.into_iter().map(|x| x.name).collect())
    }
}

impl Drop for TestClient<'_> {
    fn drop(&mut self) {
        self.client.delete_vhost(&self.vhost, true).unwrap();
    }
}

pub fn create_args(client: &TestClient, dry_run: bool) -> Args {
    Args {
        url: "http://guest:guest@localhost:15672/api".to_string(),
        vhost: client.vhost.clone(),
        dry_run,
        action: None,
    }
}

pub fn wait_for_0_messages(client: &TestClient, queue: &str) -> Result<(), Box<dyn Error>> {
    for _ in 0..10 {
        if client.get_number_of_messages(queue)? == 0 {
            return Ok(());
        }
        std::thread::sleep(Duration::from_secs(1));
    }

    panic!("Queue '{}' still has messages after waiting", queue);
}
