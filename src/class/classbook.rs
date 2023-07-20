use std::error::Error;
use reqwest::Client;
use std::result::Result;
use scraper::{ElementRef, Html, Selector};
use crate::auth::client::{extract_direct_link, get_body};
use futures::stream::FuturesUnordered;
use futures::stream::StreamExt;
use lazy_static::lazy_static;
use html_escape::decode_html_entities;
use linked_hash_set::LinkedHashSet;
use log::debug;
use crate::common::tables::{Classbook, ClassbookEntry};
use crate::utils::replacement::apply_replacements;

lazy_static! {
    // Regular expressions
    static ref LIST_GROUP_ITEM_SELECTOR: Selector = Selector::parse(r#"li.list-group-item"#).unwrap();
    static ref A_SELECTOR: Selector = Selector::parse("a").unwrap();
    static ref TABLE_SELECTOR: Selector = Selector::parse(r#"table.generaltable.attwidth.boxaligncenter tbody tr"#).unwrap();
    static ref DESCRIPTION_SELECTOR: Selector = Selector::parse(r#"td.desccol.cell.c1"#).unwrap();
    static ref DATE_AND_TIME_SELECTOR: Selector = Selector::parse("td.datecol.cell.c0").unwrap();
    static ref P_SELECTOR: Selector = Selector::parse("p").unwrap();
    static ref P_SPAN_SELECTOR: Selector = Selector::parse("p > span").unwrap();
    static ref TD_SELECTOR: Selector = Selector::parse("td.desccol.cell.c1").unwrap();
    static ref STRONG_IN_TD_SELECTOR: Selector = Selector::parse("td > strong").unwrap();
    static ref LI_SELECTOR: Selector = Selector::parse("li").unwrap();
    static ref H2_IN_TD_SELECTOR: Selector = Selector::parse("td > h2").unwrap();
    static ref TD_UL_LI_SPAN_SELECTOR: Selector = Selector::parse("td ul li span").unwrap();
}

// Extracts the classbook entries from the given HTML body
pub async fn extract_classbook(client: &Client, body: &String) -> Result<Classbook, Box<dyn Error>> {
    let document = Html::parse_document(body);

    let elements: Vec<_> = document.select(&LIST_GROUP_ITEM_SELECTOR)
        .filter(|elem| {
            let text = elem.text().collect::<String>();
            text.trim() == "Klassenbuch"
        })
        .collect();

    if elements.is_empty() {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "no classbook link found")));
    }

    let mut tasks = FuturesUnordered::new();
    for element in elements {
        let client = client.clone();

        let a_element = element.select(&A_SELECTOR).next()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "no a element found in li.list-group-item"))?;

        let link = a_element.value().attr("href")
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "classbook href not found"))?.to_string();

        let id = element.value().attr("data-key")
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "classbook data-key not found"))?.to_string();

        tasks.push(async move {
            let classbook_body = get_body(&client, &link).await?;
            let direct_link = extract_direct_link(&classbook_body)?;

            let direct_link = direct_link.ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "direct link not found"))?;

            let entries = extract_classbook_entries(&client, &[direct_link.clone()]).await?;

            debug!("Classbook -> ID {}, Link: {}, Direct Link: {}", id, link, &direct_link);

            Ok::<_, Box<dyn Error>>(Classbook { link, id, direct_link: Some(direct_link), entries })
        });
    }

    let mut results = Vec::new();
    while let Some(result) = tasks.next().await {
        match result {
            Ok(classbook) => results.push(classbook),
            Err(e) => eprintln!("Failed to extract classbook: {:?}", e),
        }
    }

    // Return the first classbook
    results.into_iter().next().ok_or_else(|| Box::new(std::io::Error::new(std::io::ErrorKind::Other, "no classbook processed")) as Box<dyn Error>)
}

pub async fn extract_classbook_entries(client: &Client, direct_links: &[String]) -> Result<Vec<ClassbookEntry>, Box<dyn Error>> {
    let mut tasks = FuturesUnordered::new();

    for link in direct_links {
        let client = client.clone();
        let link = link.clone();

        tasks.push(async move {
            let body = get_body(&client, &link).await?;
            let document = Html::parse_document(&body);
            extract_classbook_entry(document).await
        });
    }

    let mut results = Vec::new();
    while let Some(result) = tasks.next().await {
        results.push(result);
    }

    let entries = results.into_iter()
        .filter_map(|result| result.ok())
        .flatten()
        .collect::<Vec<ClassbookEntry>>();

    Ok(entries)
}

// Function to select an element from a document
fn select_element_and_extract_text<'a>(element: &'a ElementRef, selector: &Selector) -> Result<String, Box<dyn Error>> {
    let selected_element = element.select(selector).next().ok_or_else(|| format!("Element not found for selector {:?}", selector))?;
    Ok(selected_element.text().collect::<String>().trim().to_string())
}

// Function to process elements based on a given selector
async fn process_elements_by_selector(element: ElementRef<'_>, selector: &Selector) -> LinkedHashSet<String> {
    let mut activities = LinkedHashSet::new();

    for sub_element in element.select(selector).collect::<Vec<_>>() {
        let activity = sub_element.text().collect::<String>().trim().to_string();
        if !activity.is_empty() {
            activities.insert(activity);
        }
    }

    return activities;
}

// Function to process a classbook entry
async fn process_element(element: ElementRef<'_>, selectors: &[&Selector]) -> LinkedHashSet<String> {
    let mut activities = LinkedHashSet::new();

    for selector in selectors {
        for sub_element in element.select(selector).collect::<Vec<_>>() {
            let activities_html = Html::parse_fragment(&sub_element.inner_html());

            // Split activities by HTML line breaks and add each activity to the set
            for node in activities_html.root_element().children() {
                if let Some(activity) = node.value().as_text() {
                    // Decode HTML entities and trim the activity
                    let activity = apply_replacements(&decode_html_entities::<str>(activity)).await.trim().to_string();

                    // Only add the activity if it is not empty
                    if !activity.is_empty() {
                        activities.insert(activity);
                    }
                }
            }
        }
    }

    // Process <strong> tags in <td>
    activities.extend(process_elements_by_selector(element, &STRONG_IN_TD_SELECTOR).await);

    // Process <h2> tags in <td>
    activities.extend(process_elements_by_selector(element, &H2_IN_TD_SELECTOR).await);

    // Process <td ul li span> tags
    activities.extend(process_elements_by_selector(element, &TD_UL_LI_SPAN_SELECTOR).await);

    return activities;
}

// Function to parse the date and time from the HTML document
async fn extract_classbook_entry(document: Html) -> Result<Vec<ClassbookEntry>, Box<dyn Error>> {
    let mut entries = Vec::new();

    for element in document.select(&TABLE_SELECTOR) {
        let date_and_time_str = select_element_and_extract_text(&element, &DATE_AND_TIME_SELECTOR)?;
        let description = select_element_and_extract_text(&element, &DESCRIPTION_SELECTOR)?;

        let (weekday, date, mut time) = parse_date_and_time(&date_and_time_str);

        // Extract the activities from the description by selecting the <p>, <li>, and <td> elements
        let mut activities = LinkedHashSet::new();
        activities.extend(process_element(element, &[&P_SELECTOR, &LI_SELECTOR, &TD_SELECTOR, &P_SPAN_SELECTOR]).await);

        // Check if the time range is valid
        time = limit_time_range(&time);

        // Print out all activities to be able to see if the splitting worked correctly
        for activity in &activities {
            debug!("Activities -> Weekday: {}, Date: {}, Time: {}, Activity: {}", weekday, date, time, activity);
        }

        entries.push(ClassbookEntry { weekday, date, time, description, activities });
    }

    Ok(entries)
}

// Function to limit the time range to "08:00 - 16:30"
fn limit_time_range(time: &str) -> String {
    // Check if time is empty and set it to "08:00 - 16:30" if true
    if time.is_empty() {
        return "08:00 - 16:30".to_string();
    }

    // Split the time into start and end time
    let split_time: Vec<&str> = time.split(" - ").collect();
    if split_time.len() == 2 {
        let start_time = split_time[0].trim();
        let end_time = split_time[1].trim();

        // Check if end time is greater than "16:30"
        if end_time > "16:30" {
            // Set end time to "16:30"
            return format!("{} - 16:30", start_time);
        }
    }

    return time.to_string();
}

// Function to parse date and time from a string and return them as a tuple
fn parse_date_and_time(date_and_time_str: &str) -> (String, String, String) {
    let date_and_time_parts: Vec<&str> = date_and_time_str.split(',').collect();
    let weekday = date_and_time_parts[0].trim().to_string();

    let date_and_time = date_and_time_parts[1].trim();
    let date_and_time_parts: Vec<&str> = date_and_time.split(' ').collect();
    let date = date_and_time_parts[0].to_string();
    let mut time = "".to_string();

    if date_and_time_parts.len() >= 4 {
        time = format!("{} - {}", date_and_time_parts[1], date_and_time_parts[3]);
    }

    return (weekday, date, time);
}
