use leap_connect::v1::api::Client;
use leap_connect::v1::common::TEXT_EMBEDDING_3_SMALL;
use leap_connect::v1::embedding::EmbeddingRequest;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new(env::var("TUPLELEAP_AI_API_KEY").unwrap().to_string());

    let mut req =
        EmbeddingRequest::new(TEXT_EMBEDDING_3_SMALL.to_string(), "story time".to_string());
    req.dimensions = Some(10);

    let result = client.embedding(req)?;
    println!("{:?}", result.data);

    Ok(())
}

// OPENAI_API_KEY=xxxx cargo run --package openai-api-rs --example embedding
