use crate::{config::Config, context::Context};
use anyhow::{anyhow, Context as _, Result};
use chrono::{DateTime, Utc};
use derive_more::Deref;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Base URL for the OpenShock API
const OPENSHOCK_BASE_URL: &str = "https://api.openshock.app";

/// Service for interacting with the OpenShock API.
///
/// Provides methods for listing devices and sending control commands to shockers.
/// All operations require a valid API key configured in the application config.
#[derive(Debug, Clone)]
pub struct OpenshockService {
    client: reqwest::Client,
    base_url: String,
}

/// Wrapper type for shocker activation duration in milliseconds.
///
/// Enforces OpenShock API constraint that duration must be at least 300ms.
/// Maximum value is implicitly capped at 65535ms (u16::MAX).
///
/// Implements `Deref` to u16 for easy access to the underlying value.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Deref, ToSchema)]
#[repr(transparent)]
#[serde(try_from = "u16", into = "u16")]
pub struct ActionDuration(u16);

impl ActionDuration {
    /// Creates a new Duration, validating it meets API requirements.
    ///
    /// # Arguments
    /// * `ms` - Duration in milliseconds (must be >= 300)
    ///
    /// # Errors
    /// Returns an error if the duration is less than 300ms.
    pub fn new(ms: u16) -> Result<Self> {
        if ms < 300 {
            return Err(anyhow!("Duration must be at least 300 ms, got {}", ms));
        }
        Ok(ActionDuration(ms))
    }
}

impl TryFrom<u16> for ActionDuration {
    type Error = anyhow::Error;

    fn try_from(ms: u16) -> Result<Self> {
        ActionDuration::new(ms)
    }
}

impl From<ActionDuration> for u16 {
    fn from(duration: ActionDuration) -> u16 {
        duration.0
    }
}

/// Wrapper type for shocker intensity level.
///
/// Enforces OpenShock API constraint that intensity must be between 0-100 inclusive.
/// A value of 0 represents no intensity, while 100 represents maximum intensity.
///
/// Implements `Deref` to u8 for easy access to the underlying value.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Deref, ToSchema)]
#[repr(transparent)]
#[serde(try_from = "u16", into = "u16")]
pub struct ActionIntensity(u16);

impl ActionIntensity {
    /// Creates a new Intensity, validating it meets API requirements.
    ///
    /// # Arguments
    /// * `intensity` - Intensity level (must be 0-100 inclusive)
    ///
    /// # Errors
    /// Returns an error if the intensity is greater than 100.
    pub fn new(intensity: u16) -> Result<Self> {
        if intensity > 100 {
            return Err(anyhow!(
                "Intensity must be between 0 and 100, got {}",
                intensity
            ));
        }
        Ok(ActionIntensity(intensity))
    }
}

impl TryFrom<u16> for ActionIntensity {
    type Error = anyhow::Error;

    fn try_from(intensity: u16) -> Result<Self> {
        ActionIntensity::new(intensity)
    }
}

impl From<ActionIntensity> for u16 {
    fn from(intensity: ActionIntensity) -> u16 {
        intensity.0
    }
}

impl std::fmt::Display for ActionIntensity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The type of control action to send to a shocker device.
///
/// Each variant represents a different mode of operation:
/// - `Stop`: Immediately stops any ongoing activity
/// - `Shock`: Delivers an electrical shock at the specified intensity
/// - `Vibrate`: Activates vibration motor at the specified intensity  
/// - `Sound`: Plays a sound/beep at the specified intensity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ControlType {
    /// Stop any ongoing shocker activity immediately
    Stop,
    /// Deliver an electrical shock
    Shock,
    /// Activate vibration motor
    Vibrate,
    /// Play a sound/beep
    Sound,
}

/// A control message to send to a specific shocker device.
///
/// Contains all the information needed to perform an action on a shocker,
/// including the target device, action type, intensity, and duration.
#[derive(Debug, Clone, Serialize)]
pub struct ControlMsg {
    /// The UUID of the target shocker device
    pub id: Uuid,
    /// The type of control action to perform
    #[serde(rename = "type")]
    pub control_type: ControlType,
    /// The intensity level for the action (0-100)
    pub intensity: ActionIntensity,
    /// How long the action should last (minimum 300ms)
    pub duration: ActionDuration,
    /// Whether this action should be exclusive (stop other actions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusive: Option<bool>,
}

/// Represents an OpenShock device (hub) that can control multiple shockers.
///
/// A device is the main hardware unit that connects to the OpenShock service
/// and can control one or more shocker units attached to it.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    /// Unique identifier for this device
    pub id: Uuid,
    /// Human-readable name assigned to this device
    pub name: String,
    /// When this device was first registered with OpenShock
    pub created_on: DateTime<Utc>,
}

/// Represents a shocker unit that can be controlled via OpenShock.
///
/// A shocker is a physical device that can deliver various types of stimulation
/// (shock, vibration, sound) and is connected to an OpenShock device/hub.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Shocker {
    /// Unique identifier for this shocker
    pub id: Uuid,
    /// RF ID used for radio communication with the shocker
    pub rf_id: u32,
    /// The model/type of this shocker device
    pub model: String, // Note: API uses ShockerModelType enum, but we'll use String for simplicity
    /// Human-readable name assigned to this shocker
    pub name: String,
    /// Whether this shocker is currently paused (cannot receive commands)
    pub is_paused: bool,
    /// When this shocker was first registered
    pub created_on: DateTime<Utc>,
}

/// Represents a device with its associated shockers.
///
/// This is the response format when listing devices with their shockers,
/// providing a complete view of the user's OpenShock setup.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeviceWithShockers {
    /// Unique identifier for this device
    pub id: Uuid,
    /// Human-readable name assigned to this device
    pub name: String,
    /// When this device was first registered with OpenShock
    pub created_on: DateTime<Utc>,
    /// All shockers connected to this device
    pub shockers: Vec<Shocker>,
}

/// Internal wrapper for OpenShock API responses.
///
/// The OpenShock API wraps most responses in a legacy format with
/// a message field and the actual data in a `data` field.
#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    /// Status message from the API (unused)
    #[allow(dead_code)]
    message: String,
    /// The actual response data
    data: T,
}

impl OpenshockService {
    /// Creates a new OpenshockService from the application configuration.
    ///
    /// # Arguments
    /// * `config` - Application configuration containing the OpenShock API key
    ///
    /// # Returns
    /// A configured service ready to make API calls to OpenShock.
    pub fn from_config(config: &Config) -> Self {
        // Configure auth headers.
        let mut headers = HeaderMap::new();
        headers.insert(
            "OpenShockToken",
            HeaderValue::from_str(&config.openshock_api_key).unwrap(),
        );
        //headers.insert("User-Agent", "PostmanRuntime/7.45.0".parse().unwrap());
        headers.insert("User-Agent", "zap.w01.dev".parse().unwrap());

        // Build the API client.
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to build OpenShock HTTP client");

        Self {
            client,
            base_url: OPENSHOCK_BASE_URL.to_string(),
        }
    }

    /// Lists all devices registered to the authenticated user's account.
    ///
    /// # Arguments
    /// * `_ctx` - Application context (unused but follows service pattern)
    ///
    /// # Returns
    /// A vector of Device structs representing the user's OpenShock devices.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The HTTP request fails
    /// - The API returns an error status
    /// - The response cannot be parsed as JSON
    pub async fn list_devices(&self, _ctx: &Context) -> Result<Vec<Device>> {
        let url = format!("{}/1/devices", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to OpenShock API")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "OpenShock API returned error: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        let response_data: ApiResponse<Vec<Device>> = response
            .json()
            .await
            .context("Failed to parse response from OpenShock API")?;

        Ok(response_data.data)
    }

    /// Lists all shockers registered to the authenticated user's account.
    ///
    /// Returns devices with their associated shockers, preserving the original
    /// API response structure so callers can access device information and
    /// flatten shockers as needed.
    ///
    /// # Arguments
    /// * `_ctx` - Application context (unused but follows service pattern)
    ///
    /// # Returns
    /// A vector of DeviceWithShockers structs from the OpenShock API.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The HTTP request fails
    /// - The API returns an error status
    /// - The response cannot be parsed as JSON
    pub async fn list_shockers(&self, _ctx: &Context) -> Result<Vec<DeviceWithShockers>> {
        let url = format!("{}/1/shockers/own", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to OpenShock API")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "OpenShock API returned error: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        let response_data: ApiResponse<Vec<DeviceWithShockers>> = response
            .json()
            .await
            .context("Failed to parse response from OpenShock API")?;

        Ok(response_data.data)
    }

    /// Sends control commands to one or more shocker devices.
    ///
    /// # Arguments
    /// * `_ctx` - Application context (unused but follows service pattern)
    /// * `controls` - Slice of control messages to send to devices
    ///
    /// # Returns
    /// Returns `Ok(())` if all commands were successfully sent.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The HTTP request fails
    /// - The API returns an error status (e.g., device not found, insufficient permissions)
    /// - The request payload cannot be serialized
    ///
    /// # Note
    /// This method uses the v1 control endpoint. When available, consider migrating
    /// to the v2 endpoint for improved functionality.
    pub async fn control(&self, _ctx: &Context, controls: &[ControlMsg]) -> Result<()> {
        let url = format!("{}/1/shockers/control", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(controls)
            .send()
            .await
            .context("Failed to send control request to OpenShock API")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "OpenShock API control request failed: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        Ok(())
    }
}
