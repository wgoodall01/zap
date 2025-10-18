/// Minimal reproduction of utoipa bug where a type named `Duration`
/// fails to generate an OpenAPI schema, while an identically-structured
/// type with a different name works correctly.
///
/// Run with: cargo run --example utoipa_duration_bug
/// Or compile standalone: rustc --edition 2021 utoipa_duration_bug.rs

use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

/// This type generates a proper OpenAPI schema
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[repr(transparent)]
#[serde(try_from = "u16", into = "u16")]
pub struct Intensity(u16);

impl Intensity {
    pub fn new(value: u16) -> Result<Self, String> {
        if value > 100 {
            return Err(format!("Intensity must be <= 100, got {}", value));
        }
        Ok(Intensity(value))
    }
}

impl TryFrom<u16> for Intensity {
    type Error = String;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Intensity::new(value)
    }
}

impl From<Intensity> for u16 {
    fn from(intensity: Intensity) -> u16 {
        intensity.0
    }
}

/// This type is IDENTICAL to Intensity but named Duration.
/// It fails to generate an OpenAPI schema - utoipa seems to confuse it
/// with std::time::Duration and generates inline "type": "string" instead.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[repr(transparent)]
#[serde(try_from = "u16", into = "u16")]
pub struct Duration(u16);

impl Duration {
    pub fn new(value: u16) -> Result<Self, String> {
        if value < 300 {
            return Err(format!("Duration must be >= 300ms, got {}", value));
        }
        Ok(Duration(value))
    }
}

impl TryFrom<u16> for Duration {
    type Error = String;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Duration::new(value)
    }
}

impl From<Duration> for u16 {
    fn from(duration: Duration) -> u16 {
        duration.0
    }
}

/// An action that uses both types
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub enum Action {
    Shock {
        intensity: Intensity,
        duration: Duration,
    },
}

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    components(schemas(Action, Intensity, Duration))
)]
pub struct ApiDoc;

fn main() {
    let openapi = ApiDoc::openapi();
    let json = serde_json::to_string_pretty(&openapi).unwrap();

    println!("Generated OpenAPI spec:\n{}\n", json);

    // Parse the JSON to check the schemas
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let schemas = &value["components"]["schemas"];

    // Check if Duration schema exists
    let has_duration_schema = schemas.get("Duration").is_some();
    let has_intensity_schema = schemas.get("Intensity").is_some();

    println!("=== Test Results ===");
    println!("Intensity schema exists: {}", has_intensity_schema);
    println!("Duration schema exists: {}", has_duration_schema);

    if has_intensity_schema {
        let intensity_type = schemas["Intensity"]["type"].as_str();
        println!("Intensity type in schema: {:?}", intensity_type);
        assert_eq!(intensity_type, Some("integer"), "Intensity should be integer type");
    } else {
        panic!("FAIL: Intensity schema missing!");
    }

    // Check what happens to duration in the Action enum
    let action_schema = &schemas["Action"];
    let shock_variant = &action_schema["oneOf"][0]["properties"]["Shock"]["properties"];
    let duration_field = &shock_variant["duration"];
    let intensity_field = &shock_variant["intensity"];

    println!("\nIn Action.Shock variant:");
    println!("  intensity field: {}", serde_json::to_string_pretty(&intensity_field).unwrap());
    println!("  duration field: {}", serde_json::to_string_pretty(&duration_field).unwrap());

    // Check if duration has a $ref or is inlined
    let duration_has_ref = duration_field.get("$ref").is_some();
    let duration_inline_type = duration_field.get("type").and_then(|v| v.as_str());
    let intensity_has_ref = intensity_field.get("$ref").is_some();

    println!("\n=== Bug Demonstration ===");
    println!("intensity has $ref: {} (expected: true)", intensity_has_ref);
    println!("duration has $ref: {} (expected: true)", duration_has_ref);

    if !duration_has_ref && duration_inline_type.is_some() {
        println!("\nBUG CONFIRMED: Duration is inlined as type '{}' instead of using $ref",
                 duration_inline_type.unwrap());
        println!("This happens even though Duration has #[derive(ToSchema)] and is identical to Intensity");
        println!("\nLikely cause: utoipa is confusing our custom Duration type with std::time::Duration");
    }

    if has_duration_schema {
        println!("\nDuration schema does exist in components, but it's not being referenced!");
        let duration_type = schemas["Duration"]["type"].as_str();
        println!("Duration type in schema: {:?}", duration_type);
    } else {
        println!("\nDuration schema is completely missing from components!");
    }

    // Final assertion
    assert_eq!(
        duration_has_ref, intensity_has_ref,
        "BUG: Duration and Intensity should both use $ref (they are identically structured), \
         but Duration has $ref={} while Intensity has $ref={}",
        duration_has_ref, intensity_has_ref
    );
}
