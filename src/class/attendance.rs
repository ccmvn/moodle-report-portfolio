use std::error::Error;
use std::sync::Arc;
use log::debug;
use scraper::{Html, Selector};
use reqwest::Client;
use crate::auth::client::get_body;
use crate::common::tables::Attendance;
use crate::CONFIG;

const ATTENDANCE_PATH: &str = "/local/anmeldung/anwesenheit.php?page=1";

// This function scrapes the attendance from the course attendance page
pub async fn scrape_attendance(client: Arc<Client>) -> Result<Vec<Attendance>, Box<dyn Error>> {
    let attendance_url = format!("{}{}", CONFIG.get_base_url(), ATTENDANCE_PATH);
    let body = get_body(&*client, &attendance_url).await?;
    extract_attendance(&body)
}

// This function extracts the attendance information from the HTML content of the page
pub fn extract_attendance(body: &str) -> Result<Vec<Attendance>, Box<dyn Error>> {
    let fragment = Html::parse_document(body);

    // Define selectors
    let table_selector = Selector::parse(".table")?;
    let row_selector = Selector::parse("tr")?;
    let cell_selector = Selector::parse("td")?;

    // Find the table
    let table = fragment.select(&table_selector).next().ok_or("Attendance table not found")?;

    // Loop over each row in the table
    let mut attendance_records = Vec::new();
    let mut rows = table.select(&row_selector);

    // Skip the first row (header)
    if let Some(_) = rows.next() {
        for row_element in rows {
            let cells: Vec<_> = row_element.select(&cell_selector).collect();

            // Collect the cells into a vector
            let mut cell_contents: Vec<String> = Vec::new();
            for cell in &cells {
                cell_contents.push(cell.inner_html().trim().to_string());
            }

            // Create an attendance record
            let attendance = Attendance {
                date: cells[1].text().collect::<String>().trim().to_string(),
                from_time: cells[2].text().collect::<String>().trim().to_string(),
                to_time: cells[3].text().collect::<String>().trim().to_string(),
            };
            attendance_records.push(attendance);

            debug!("Attendance -> Date: {}, From: {}, To: {}",
                  cell_contents[1], cell_contents[2], cell_contents[3]);
        }
    }

    Ok(attendance_records)
}
