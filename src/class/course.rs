use std::error::Error;
use log::{debug, error, info};
use reqwest::{Client};
use scraper::{Html, Selector};
use crate::{CONFIG};
use crate::auth::client::get_body;
use futures::{stream, StreamExt};
use tokio::time::{sleep, Duration};
use crate::class::classbook::extract_classbook;
use std::sync::Arc;
use crate::common::tables::{Classbook, Course};

const COURSE_PATH: &str = "/course/view.php?id=";

// Scrapes all courses
pub async fn scrape_courses(client: Arc<Client>) -> Result<Vec<Course>, Box<dyn Error>> {
    let body = get_body(&*client, &format!("{}/", CONFIG.get_base_url())).await?;
    let mut courses = match extract_courses(&body) {
        Ok(courses) => courses,
        Err(e) => {
            error!("Failed to extract courses: {}", e);
            return Err(e);
        }
    };

    if CONFIG.get_test_mode() {
        info!("Running in test mode, only scraping the first course");
        courses.truncate(1); // Only scrape the first course in test mode
    }

    let max_concurrent_tasks = 100; // You can tune this number to the desired level of concurrency
    let temp_results: Vec<Result<Course, _>> = stream::iter(courses.into_iter().map(|course| {
        let client = Arc::clone(&client);
        let course_link = course.link.clone();

        async move {
            sleep(Duration::from_millis(100)).await; // Add a small delay between requests to avoid overwhelming the server
            let classbook = match scrape_classbook(client.clone(), &course_link).await {
                Ok(classbook) => classbook,
                Err(e) => {
                    error!("Failed to scrape classbook: {}", e);
                    return Err(e);
                }
            };
            let mut course = course;
            course.classbook = classbook;

            Ok(course)
        }
    })).buffer_unordered(max_concurrent_tasks).collect().await;

    let results: Result<Vec<Course>, _> = temp_results.into_iter().collect();
    results
}

// Scrapes the classbook for a course
pub async fn scrape_classbook(client: Arc<Client>, course_link: &str) -> Result<Classbook, Box<dyn Error>> {
    let body = get_body(&*client, course_link).await?;
    extract_classbook(&*client, &body).await
}

// Extracts the courses from the body
pub fn extract_courses(body: &str) -> Result<Vec<Course>, Box<dyn Error>> {
    let fragment = Html::parse_document(body);

    let course_id_selector = Selector::parse("[data-courseid]")?;
    let course_name_selector = Selector::parse(".card-title")?;

    let mut courses = Vec::new();

    for element in fragment.select(&course_id_selector) {
        if let Some(course_id) = element.value().attr("data-courseid") {
            // Get the first element that matches the course name selector
            if let Some(course_name_element) = element.select(&course_name_selector).next() {
                let course_name = course_name_element.text().collect::<String>().trim().to_string(); // Get the course name
                let course_parts: Vec<&str> = course_name.split_whitespace().collect(); // Split the course name by whitespace
                let len = course_parts.len();

                if len < 2 || !course_parts[0].starts_with("LF") {
                    continue; // Skip this course if it doesn't match the expected format or the course doesn't start with "LF"
                }

                let course = String::from(course_parts[0]); // Get the first part of the course name
                let duration = String::from(course_parts[len - 1]); // Get the last part of the course name
                let name = course_parts[1..len - 1].join(" "); // Join the remaining parts of the course name
                let link = generate_course_link(course_id); // Generate the course link

                debug!("Course -> ID: {}, Name: {}, Lernfeld: {}, Duration: {}, Link: {}",
                    course_id, name, course, duration, link);

                let course = Course {
                    id: course_id.to_string(),
                    name,
                    link,
                    course,
                    duration,
                    classbook: Classbook { id: String::new(), link: String::new(), direct_link: Some(String::new()), entries: Vec::new() },
                };

                courses.push(course);
            }
        }
    }

    Ok(courses)
}

// Generate the course link from the course ID
pub fn generate_course_link(course_id: &str) -> String {
    format!("{}{}{}", CONFIG.get_base_url(), COURSE_PATH, course_id)
}
