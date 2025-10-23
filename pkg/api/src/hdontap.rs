use anyhow::{Result, anyhow};
use regex::Regex;
use scraper::{Html, Selector};
use serde_json::Value;
use std::fmt;
use std::str::FromStr;
use url::Url;

/// A validated HDOnTap stream ID
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamId(String);

impl StreamId {
    /// Create a StreamId from a u64
    pub fn from_u64(id: u64) -> Self {
        Self(id.to_string())
    }

    /// Create a StreamId from a URL, extracting the numeric ID
    ///
    /// Accepts various URL formats:
    /// - https://hdontap.com/stream/123456/wolf-ambassadors
    /// - http://hdontap.com/stream/123456
    /// - hdontap.com/stream/123456/slug?params=1
    /// - https://hdontap.com/stream/123456
    pub fn from_url(url: &str) -> Result<Self> {
        // Match with optional scheme (http/https), optional trailing slash, and anything after the ID
        let url_pattern = Regex::new(r"(?:https?://)?hdontap\.com/stream/(\d+)")?;
        let captures = url_pattern
            .captures(url)
            .ok_or_else(|| anyhow!("Invalid HDOnTap URL format"))?;

        let id = captures
            .get(1)
            .ok_or_else(|| anyhow!("Could not extract stream ID from URL"))?
            .as_str();

        Ok(Self(id.to_string()))
    }

    /// Get the raw string representation of the stream ID
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for StreamId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        // First try to parse as URL
        if let Ok(stream_id) = StreamId::from_url(s) {
            return Ok(stream_id);
        }

        // Otherwise, validate as numeric ID
        let id_pattern = Regex::new(r"^\d+$")?;
        if id_pattern.is_match(s) {
            Ok(Self(s.to_string()))
        } else {
            Err(anyhow!(
                "Invalid stream ID format: must be numeric or valid HDOnTap URL"
            ))
        }
    }
}

impl fmt::Display for StreamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct HdontapService;

impl Default for HdontapService {
    fn default() -> Self {
        Self::new()
    }
}

impl HdontapService {
    pub fn new() -> Self {
        Self
    }

    /// Get the M3U8 stream URL for a given stream ID
    #[tracing::instrument(name = "HdontapService::get_stream_url", skip(self), fields(stream_id = %stream_id))]
    pub async fn get_stream_url(&self, stream_id: &StreamId) -> Result<Url> {
        let html_content = self.fetch_stream_page(stream_id).await?;
        let player_data = self.extract_player_data(&html_content)?;

        match player_data {
            Some(data) => {
                let m3u8_url = self.get_m3u8_url(&data)?;
                Ok(Url::parse(&m3u8_url)?)
            }
            None => Err(anyhow!("Stream is currently offline")),
        }
    }

    /// Fetch the stream page HTML
    #[tracing::instrument(name = "HdontapService::fetch_stream_page", skip(self), fields(stream_id = %stream_id))]
    async fn fetch_stream_page(&self, stream_id: &StreamId) -> Result<String> {
        let url = format!("https://hdontap.com/stream/{}/", stream_id);

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("User-Agent", "Wolp (github.com/wgoodall01/zap)")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch stream page: HTTP {}",
                response.status()
            ));
        }

        Ok(response.text().await?)
    }

    /// Extract JSON from the player-data script tag using HTML parser
    /// Returns None if the stream is offline
    fn extract_player_data(&self, html_content: &str) -> Result<Option<Value>> {
        let document = Html::parse_document(html_content);
        let selector = Selector::parse(r#"script[id="player-data"]"#)
            .map_err(|e| anyhow!("Invalid CSS selector: {}", e))?;

        // Try to find the player-data script tag
        if let Some(script_element) = document.select(&selector).next() {
            let json_content = script_element.inner_html();
            let player_data: Value = serde_json::from_str(&json_content)
                .map_err(|e| anyhow!("Error parsing JSON: {}", e))?;
            return Ok(Some(player_data));
        }

        // If no player-data tag found, check if stream is offline
        let stream_player_selector = Selector::parse(".stream-player")
            .map_err(|e| anyhow!("Invalid CSS selector: {}", e))?;

        if let Some(stream_player) = document.select(&stream_player_selector).next() {
            let html = stream_player.html();
            if html.contains("Currently Offline") {
                return Ok(None);
            }
        }

        // Neither player-data nor offline indicator found
        Err(anyhow!(
            "Could not find player-data script tag or offline indicator"
        ))
    }

    /// Extract the M3U8 URL from player data
    fn get_m3u8_url(&self, player_data: &Value) -> Result<String> {
        let stream_src = player_data
            .get("streamSrc")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("No streamSrc found in player data"))?;

        Ok(stream_src.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_stream_id_from_u64() {
        let stream_id = StreamId::from_u64(259696);
        assert_eq!(stream_id.as_str(), "259696");
        assert_eq!(stream_id.to_string(), "259696");
    }

    #[test]
    fn test_stream_id_from_url() {
        // Test with https URL with trailing slash and slug
        let result = StreamId::from_url("https://hdontap.com/stream/259696/wolf-ambassadors");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "259696");

        // Test with https URL without trailing slash
        let result = StreamId::from_url("https://hdontap.com/stream/123456");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "123456");

        // Test with http URL
        let result = StreamId::from_url("http://hdontap.com/stream/999888");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "999888");

        // Test without scheme
        let result = StreamId::from_url("hdontap.com/stream/777666");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "777666");

        // Test with query parameters
        let result = StreamId::from_url("https://hdontap.com/stream/555444/slug?params=1&foo=bar");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "555444");

        // Test with just trailing slash
        let result = StreamId::from_url("https://hdontap.com/stream/333222/");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "333222");

        // Test with invalid URL
        let result = StreamId::from_url("https://example.com/invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_stream_id_from_str() {
        // Test parsing numeric ID
        let result = "259696".parse::<StreamId>();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "259696");

        // Test parsing full URL
        let result = "https://hdontap.com/stream/123456/wolf-ambassadors".parse::<StreamId>();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "123456");

        // Test invalid formats
        assert!("not-a-number".parse::<StreamId>().is_err());
        assert!("".parse::<StreamId>().is_err());
        assert!("https://example.com/invalid".parse::<StreamId>().is_err());
    }

    #[test]
    fn test_extract_player_data_valid_html() {
        let service = HdontapService::new();

        let html = r#"
            <html>
            <head><title>Test</title></head>
            <body>
                <script id="player-data" type="application/json">
                {"streamSrc": "https://live.hdontap.com/hls/test.m3u8", "other": "data"}
                </script>
            </body>
            </html>
        "#;

        let result = service.extract_player_data(html);
        assert!(result.is_ok());

        let player_data = result.unwrap();
        assert!(player_data.is_some());
        let data = player_data.unwrap();
        assert_eq!(data["streamSrc"], "https://live.hdontap.com/hls/test.m3u8");
        assert_eq!(data["other"], "data");
    }

    #[test]
    fn test_extract_player_data_missing_script() {
        let service = HdontapService::new();

        let html = r#"
            <html>
            <head><title>Test</title></head>
            <body>
                <p>No player data here</p>
            </body>
            </html>
        "#;

        let result = service.extract_player_data(html);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Could not find player-data script tag or offline indicator")
        );
    }

    #[test]
    fn test_extract_player_data_invalid_json() {
        let service = HdontapService::new();

        let html = r#"
            <html>
            <head><title>Test</title></head>
            <body>
                <script id="player-data" type="application/json">
                {"invalid": json syntax}
                </script>
            </body>
            </html>
        "#;

        let result = service.extract_player_data(html);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Error parsing JSON")
        );
    }

    #[test]
    fn test_get_m3u8_url_valid_data() {
        let service = HdontapService::new();

        let player_data = json!({
            "streamSrc": "https://live.hdontap.com/hls/test.m3u8?t=token&e=expiry",
            "otherField": "value"
        });

        let result = service.get_m3u8_url(&player_data);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "https://live.hdontap.com/hls/test.m3u8?t=token&e=expiry"
        );
    }

    #[test]
    fn test_get_m3u8_url_missing_stream_src() {
        let service = HdontapService::new();

        let player_data = json!({
            "otherField": "value",
            "noStreamSrc": true
        });

        let result = service.get_m3u8_url(&player_data);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No streamSrc found in player data")
        );
    }

    #[test]
    fn test_get_m3u8_url_stream_src_not_string() {
        let service = HdontapService::new();

        let player_data = json!({
            "streamSrc": 12345
        });

        let result = service.get_m3u8_url(&player_data);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No streamSrc found in player data")
        );
    }

    #[test]
    fn test_extract_player_data_offline_stream() {
        let service = HdontapService::new();

        let html = r#"
            <html>
            <head><title>Test</title></head>
            <body>
                <div class="bg-base-100 stream-player portrait:sticky landscape:relative z-20 portrait:top-[calc(var(--app-header-mobile-height)+var(--stream-page-header-ad-height))]">
                    <div class="text-2xl">
                        <div class="text-gray-50">
                            <div class="bg-black w-full aspect-video flex flex-col justify-center items-center rounded-md">
                                <img class="w-36 h-16 -mt-12" src="/static/mainsite/hdontap-rgb.4d7aa26da223.svg" alt="HDOnTap Company Logo">
                                <span class="text-[1.6rem] italic text-white">Currently Offline</span>
                            </div>
                        </div>
                    </div>
                </div>
            </body>
            </html>
        "#;

        let result = service.extract_player_data(html);
        assert!(result.is_ok());
        let player_data = result.unwrap();
        assert!(player_data.is_none(), "Expected None for offline stream");
    }

    #[test]
    fn test_get_stream_url_offline_error() {
        // Test that when extract_player_data returns None, get_stream_url returns the correct error
        let service = HdontapService::new();

        let html = r#"
            <html>
            <body>
                <div class="stream-player">
                    <span>Currently Offline</span>
                </div>
            </body>
            </html>
        "#;

        let player_data_result = service.extract_player_data(html);
        assert!(player_data_result.is_ok());
        assert!(player_data_result.unwrap().is_none());

        // Note: We can't directly test get_stream_url without mocking the HTTP request,
        // but we can verify the logic path by checking extract_player_data returns None
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_hdontap_endpoint() {
        // This test hits the actual HDOnTap website
        // Run with: cargo test -- --ignored
        let service = HdontapService::new();
        let stream_id = StreamId::from_u64(259696); // Wolf ambassadors stream

        let result = service.get_stream_url(&stream_id).await;
        assert!(
            result.is_ok(),
            "Failed to get stream URL: {:?}",
            result.err()
        );

        let m3u8_url = result.unwrap();
        assert!(m3u8_url.as_str().contains("live.hdontap.com"));
        assert!(m3u8_url.as_str().contains(".m3u8"));
        assert!(
            m3u8_url.query().is_some(),
            "Expected query parameters (t, e)"
        );

        println!("✅ Successfully fetched M3U8 URL: {}", m3u8_url);
    }
}
