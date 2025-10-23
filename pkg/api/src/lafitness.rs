use anyhow::{Context as _, Result, anyhow};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use chrono_tz::America::Los_Angeles;
use regex::Regex;
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
    /// The check-in date/time in UTC
    pub datetime: DateTime<Utc>,
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
            return Err(anyhow!("Failed to fetch login page: {}", response.status()));
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

            if let Some(element) = document.select(&selector).next()
                && let Some(value) = element.value().attr("value")
            {
                form_data.insert(field_name.to_string(), value.to_string());
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

        let html = response
            .text()
            .await
            .context("Failed to read check-in page")?;

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

            let row_selector = Selector::parse("tr")
                .map_err(|e| anyhow!("Failed to parse row selector: {:?}", e))?;
            let cell_selector = Selector::parse("td")
                .map_err(|e| anyhow!("Failed to parse cell selector: {:?}", e))?;

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

    /// Parses a check-in date/time string to a UTC DateTime.
    ///
    /// Example input: "10/16/2025 18:26 PM PST"
    ///
    /// # Arguments
    /// * `date_str` - The date/time string from LA Fitness
    ///
    /// # Returns
    /// UTC DateTime
    ///
    /// # LA Fitness DateTime Format Quirk
    /// LA Fitness uses a non-standard datetime format where the hour appears to be
    /// in a modified 12-hour format but with values 13-24 for PM times:
    /// - "14:10 PM" means 2:10 PM (14 - 12 = 2)
    /// - "17:27 PM" means 5:27 PM (17 - 12 = 5)
    /// - AM times appear to use standard format (e.g., "10:00 AM" means 10:00 AM)
    fn parse_checkin_datetime(date_str: &str) -> Result<DateTime<Utc>> {
        // Format: "10/16/2025 18:26 PM PST"
        let re = Regex::new(r"^(\d{1,2})/(\d{1,2})/(\d{4})\s+(\d{1,2}):(\d{2})\s+(AM|PM)")
            .context("Failed to create regex")?;

        let captures = re
            .captures(date_str)
            .ok_or_else(|| anyhow!("Invalid date format: {}", date_str))?;

        let month: u32 = captures[1].parse().context("Invalid month")?;
        let day: u32 = captures[2].parse().context("Invalid day")?;
        let year: i32 = captures[3].parse().context("Invalid year")?;
        let hour: u32 = captures[4].parse().context("Invalid hour")?;
        let minute: u32 = captures[5].parse().context("Invalid minute")?;
        // captures[6] is the AM/PM marker, but LA Fitness already uses 24-hour format

        // Handle LA Fitness's weird format:
        // - "14:10 PM" means 2:10 PM -> in 24-hour format that's 14:10 (hour stays as-is)
        // - "17:27 PM" means 5:27 PM -> in 24-hour format that's 17:27 (hour stays as-is)
        // - "10:00 AM" means 10:00 AM -> in 24-hour format that's 10:00 (hour stays as-is)
        // The hour value is ALREADY in 24-hour format, so we don't need any conversion!
        let hour_24 = hour;

        // Create naive datetime
        let naive_date = NaiveDate::from_ymd_opt(year, month, day)
            .ok_or_else(|| anyhow!("Invalid date: {}/{}/{}", year, month, day))?;
        let naive_datetime = naive_date
            .and_hms_opt(hour_24, minute, 0)
            .ok_or_else(|| anyhow!("Invalid time: {}:{}", hour_24, minute))?;

        // LA Fitness reports times in Pacific Time
        // Use chrono-tz to properly handle DST transitions
        let datetime_pt = Los_Angeles
            .from_local_datetime(&naive_datetime)
            .single()
            .ok_or_else(|| anyhow!("Ambiguous or invalid local time: {}", naive_datetime))?;

        // Convert to UTC
        Ok(datetime_pt.with_timezone(&Utc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use chrono_tz::America::New_York;

    #[test]
    fn test_parse_checkin_datetime_october_edt() {
        // Parsed: 10/18/2025 14:10 PM PST
        // ET: 2025-10-18 5:10p EDT (UTC-4)
        let parsed_str = "10/18/2025 14:10 PM PST";
        let parsed =
            LaFitnessService::parse_checkin_datetime(parsed_str).expect("Failed to parse datetime");

        // Reference time: 2025-10-18 5:10p EDT
        let reference_naive = NaiveDate::from_ymd_opt(2025, 10, 18)
            .unwrap()
            .and_hms_opt(17, 10, 0) // 5:10 PM in 24-hour format
            .unwrap();
        let reference_et = New_York
            .from_local_datetime(&reference_naive)
            .single()
            .unwrap();
        let reference_utc = reference_et.with_timezone(&Utc);

        assert_eq!(
            parsed, reference_utc,
            "Parsed datetime should match reference ET datetime"
        );
    }

    #[test]
    fn test_parse_checkin_datetime_january_est() {
        // Parsed: 01/01/2026 17:27 PM PST
        // ET: 2026-01-01 8:27p EST (UTC-5)
        let parsed_str = "01/01/2026 17:27 PM PST";
        let parsed =
            LaFitnessService::parse_checkin_datetime(parsed_str).expect("Failed to parse datetime");

        // Reference time: 2026-01-01 8:27p EST
        let reference_naive = NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(20, 27, 0) // 8:27 PM in 24-hour format
            .unwrap();
        let reference_et = New_York
            .from_local_datetime(&reference_naive)
            .single()
            .unwrap();
        let reference_utc = reference_et.with_timezone(&Utc);

        assert_eq!(
            parsed, reference_utc,
            "Parsed datetime should match reference ET datetime"
        );
    }
}
