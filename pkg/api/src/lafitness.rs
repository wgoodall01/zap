use anyhow::{anyhow, Context as _, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Service for interacting with the LA Fitness website.
///
/// Provides methods for authenticating and retrieving check-in history.
#[derive(Debug, Clone)]
pub struct LaFitnessService {
    client: reqwest::Client,
}

/// Represents a single check-in at an LA Fitness location.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CheckIn {
    /// The check-in date/time in RFC 3339 UTC format
    pub datetime: String,
    /// The raw date/time string from the LA Fitness website (e.g., "10/16/2025 18:26 PM PST")
    pub datetime_raw: String,
    /// The name of the LA Fitness location
    pub location: String,
}

impl LaFitnessService {
    /// Authenticates with LA Fitness and creates a new service instance.
    ///
    /// # Arguments
    /// * `username` - LA Fitness account username
    /// * `password` - LA Fitness account password
    ///
    /// # Returns
    /// A service instance with an authenticated session ready to fetch check-ins.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The HTTP request fails
    /// - Login credentials are invalid
    /// - The login page cannot be parsed
    #[tracing::instrument(name = "LaFitnessService::login", skip(password), fields(username))]
    pub async fn login(username: String, password: String) -> Result<Self> {
        // Create a client with cookie store to maintain session
        let mut headers = HeaderMap::new();
        headers.insert(
            "User-Agent",
            HeaderValue::from_static("LaFitness (github.com/wgoodall01/zap)"),
        );

        let client = reqwest::Client::builder()
            .cookie_store(true)
            .default_headers(headers)
            .build()
            .context("Failed to build HTTP client")?;

        // Fetch the login page
        let login_page_url = "https://lafitness.com/Pages/Login.aspx";
        let response = client
            .get(login_page_url)
            .send()
            .await
            .context("Failed to fetch login page")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch login page: {}",
                response.status()
            ));
        }

        let html = response.text().await.context("Failed to read login page")?;

        // Extract ASP.NET form fields (parse and extract in one scope)
        let form_data = {
            let document = Html::parse_document(&html);
            Self::extract_form_data(&document)?
        };

        // Build the login form data
        let mut login_form = form_data;
        login_form.insert(
            "ctl00$MainContent$Login1$txtUser".to_string(),
            username.clone(),
        );
        login_form.insert(
            "ctl00$MainContent$Login1$txtPassword".to_string(),
            password.clone(),
        );
        login_form.insert(
            "ctl00$MainContent$Login1$btnLogin".to_string(),
            "Sign in".to_string(),
        );

        // Submit the login form
        let login_response = client
            .post(login_page_url)
            .form(&login_form)
            .send()
            .await
            .context("Failed to submit login")?;

        // Check if login was successful by looking at the redirect URL
        let final_url = login_response.url().as_str();
        if !final_url.to_lowercase().contains("myfitness") {
            return Err(anyhow!(
                "Login failed. Redirected to: {}. Please check credentials.",
                final_url
            ));
        }

        Ok(Self { client })
    }

    /// Extracts ASP.NET hidden form fields from the login page.
    fn extract_form_data(document: &Html) -> Result<std::collections::HashMap<String, String>> {
        let mut form_data = std::collections::HashMap::new();

        let hidden_fields = [
            "__VIEWSTATE",
            "__VIEWSTATEGENERATOR",
            "__EVENTVALIDATION",
            "__CSRFTOKEN",
        ];

        for field_name in hidden_fields {
            let selector =
                Selector::parse(&format!("input[id='{}']", field_name)).map_err(|e| {
                    anyhow!("Failed to parse selector for field {}: {:?}", field_name, e)
                })?;

            if let Some(element) = document.select(&selector).next() {
                if let Some(value) = element.value().attr("value") {
                    form_data.insert(field_name.to_string(), value.to_string());
                }
            }
        }

        Ok(form_data)
    }

    /// Fetches check-in history from LA Fitness.
    ///
    /// # Returns
    /// A vector of CheckIn records, sorted most-recent-first.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The HTTP request fails
    /// - The session is invalid (not logged in)
    /// - The check-in history page cannot be parsed
    #[tracing::instrument(name = "LaFitnessService::get_checkins", skip(self))]
    pub async fn get_checkins(&self) -> Result<Vec<CheckIn>> {
        let checkin_url = "https://lafitness.com/Pages/checkinhistory.aspx";

        let response = self
            .client
            .get(checkin_url)
            .send()
            .await
            .context("Failed to fetch check-in history")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch check-in history: {}",
                response.status()
            ));
        }

        let html = response.text().await.context("Failed to read check-in page")?;

        // Parse and extract check-ins in one scope to avoid Send issues
        let checkins = {
            let document = Html::parse_document(&html);

            // Find the check-in history table
            let table_selector =
                Selector::parse("table[id='ctl00_MainContent_checkinHistoryGrid']")
                    .map_err(|e| anyhow!("Failed to parse table selector: {:?}", e))?;

            let table = document
                .select(&table_selector)
                .next()
                .ok_or_else(|| anyhow!("Could not find check-in history table"))?;

            let row_selector =
                Selector::parse("tr").map_err(|e| anyhow!("Failed to parse row selector: {:?}", e))?;
            let cell_selector =
                Selector::parse("td").map_err(|e| anyhow!("Failed to parse cell selector: {:?}", e))?;

            let mut checkins = Vec::new();

            // Parse table rows (skip header row)
            for (idx, row) in table.select(&row_selector).enumerate() {
                if idx == 0 {
                    // Skip header row
                    continue;
                }

                let cells: Vec<_> = row.select(&cell_selector).collect();
                if cells.len() >= 2 {
                    let date_str = cells[0].text().collect::<String>().trim().to_string();
                    let location = cells[1].text().collect::<String>().trim().to_string();

                    let datetime_utc = Self::parse_checkin_datetime(&date_str)?;

                    checkins.push(CheckIn {
                        datetime: datetime_utc,
                        datetime_raw: date_str,
                        location,
                    });
                }
            }

            checkins
        };

        Ok(checkins)
    }

    /// Parses a check-in date/time string to RFC 3339 UTC timestamp.
    ///
    /// Example input: "10/16/2025 18:26 PM PST"
    ///
    /// # Arguments
    /// * `date_str` - The date/time string from LA Fitness
    ///
    /// # Returns
    /// RFC 3339 formatted UTC datetime string, or the original string if parsing fails
    fn parse_checkin_datetime(date_str: &str) -> Result<String> {
        // Format: "10/16/2025 18:26 PM PST"
        let parts: Vec<&str> = date_str.split_whitespace().collect();
        if parts.len() < 3 {
            // If we can't parse it, just return the original string
            return Ok(date_str.to_string());
        }

        let date_part = parts[0]; // "10/16/2025"
        let time_part = parts[1]; // "18:26"
        let meridiem = parts[2]; // "PM"

        // Parse the datetime string
        let dt_str = format!("{} {} {}", date_part, time_part, meridiem);

        // Try to parse the datetime
        let parsed = NaiveDateTime::parse_from_str(&dt_str, "%m/%d/%Y %H:%M %p");

        match parsed {
            Ok(dt) => {
                // LA Fitness reports times in PST (UTC-8)
                // Add 8 hours to convert PST to UTC (PST is UTC-8)
                let pst_offset = chrono::Duration::hours(8);
                let utc_dt = dt + pst_offset;
                let datetime_utc: DateTime<Utc> = DateTime::from_naive_utc_and_offset(utc_dt, Utc);
                Ok(datetime_utc.to_rfc3339())
            }
            Err(_) => {
                // If parsing fails, return the original string
                Ok(date_str.to_string())
            }
        }
    }
}
