use api::http_server::ApiDoc;
use utoipa::OpenApi;

fn main() {
    let spec = ApiDoc::openapi().to_pretty_json().unwrap();
    println!("{}", spec);
}
