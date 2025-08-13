use vectorservice::vector_service_client::VectorServiceClient;
use vectorservice::JsonRequest;

pub mod vectorservice {
    tonic::include_proto!("vectorservice");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = VectorServiceClient::connect("http://[::1]:50051").await?;

    let json_data = r#"{"key": "value"}"#;
    let request = tonic::Request::new(JsonRequest {
        json_data: json_data.to_string(),
    });

    let response = client.process_json(request).await?;
    println!("Received vector: {:?}", response.into_inner().vector);

    Ok(())
}