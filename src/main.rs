mod auth;
mod class;
mod common;
mod excel;
mod utils;

use futures::future::try_join_all;
use crate::utils::config::Config;
use lazy_static::lazy_static;
use log::{error, info};
use std::sync::Arc;
use crate::auth::client::create_client;
use crate::auth::cookies::save_cookies;
use crate::auth::login::login;
use crate::class::attendance::scrape_attendance;
use crate::class::course::scrape_courses;
use crate::excel::process::process_course;
use crate::utils::logger::setup_logger;

lazy_static! {
    static ref CONFIG: Config = Config::new().unwrap();
}

#[tokio::main]
async fn main() -> Result<(), String> {
    // Configure the logger
    match setup_logger() {
        Ok(()) => info!("Logger set up successfully"),
        Err(e) => error!("Failed to setup logger: {:?}", e)
    }

    // Welcome message
    info!("Welcome to the GFN Lernplattform Scraper!");
    info!("This program is licensed under the GNU General Public License.");
    info!("Please report any bugs to Marvin Juraschka (info@ccmvn.co)");
    // GitHub
    info!("GitHub: https://github.com/ccmvn/moodle-report-portfolio");

    // Create the client and cookie store
    let (client, cookie_store) = create_client().await.map_err(|e| format!("Failed to create the client: {}", e))?;

    // Login to the platform
    if let Ok(_) = login(&client).await {
        info!("Logged in successfully");
        info!("Read courses and create XLSX file...");

        // Wrap the Client in an Arc
        let client = Arc::new(client);

        // Scrape the global attendance information
        let attendances = scrape_attendance(Arc::clone(&client)).await.map_err(|e| format!("Failed to scrape attendance: {}", e))?;

        // Scrape the courses
        let courses = scrape_courses(Arc::clone(&client)).await;
        // Process the courses
        let course_futures = courses.iter().map(|course| { process_course(course, &attendances) });

        // Wait for all courses to be processed
        if let Err(e) = try_join_all(course_futures).await {
            return Err(format!("Failed to process courses: {}", e));
        }

        // Save the cookies
        if let Err(e) = save_cookies(cookie_store) {
            return Err(format!("Failed to save the cookies: {}", e));
        }

    } else {
        return Err("Failed to login: Invalid credentials".to_string());
    }

    Ok(())
}
