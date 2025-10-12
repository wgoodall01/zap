use crate::hdontap::{HdontapService, StreamId};
use rocket::http::Status;
use rocket::{get, response::Redirect};
use rocket_okapi::openapi;

#[openapi(tag = "Wolp", operation_id = "wolp:stream")]
#[get("/wolp/stream?<id>")]
pub async fn stream(id: String) -> Result<Redirect, Status> {
    // Parse the id as StreamId (accepts both numeric IDs and URLs)
    let stream_id = match id.parse::<StreamId>() {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Invalid stream ID or URL format: {:?}", e);
            return Err(Status::BadRequest);
        }
    };

    // Get the M3U8 URL using HdontapService
    let hdontap_service = HdontapService::new();
    match hdontap_service.get_stream_url(&stream_id).await {
        Ok(m3u8_url) => {
            eprintln!("Redirecting to: {}", m3u8_url);
            Ok(Redirect::found(m3u8_url.to_string()))
        }
        Err(e) => {
            eprintln!("Stream not found or error retrieving stream: {:?}", e);
            Err(Status::NotFound)
        }
    }
}