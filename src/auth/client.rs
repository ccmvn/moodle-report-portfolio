use std::error::Error;
use std::sync::Arc;
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Client;
use reqwest_cookie_store::CookieStoreMutex;
use scraper::{Html, Selector};
use crate::auth::cookies::load_cookie_store;

const ATTENDANCE_LINK_SELECTOR: &str = r#"a[href^="https://lernplattform.gfn.de/mod/attendance/view.php?id="]"#;

pub async fn create_client() -> Result<(Client, Arc<CookieStoreMutex>), Box<dyn std::error::Error>> {
    let cookie_store = load_cookie_store();
    let client = Client::builder()
        .cookie_provider(Arc::clone(&cookie_store))
        .gzip(true)
        .build()?;

    Ok((client, cookie_store))
}

lazy_static! {
    static ref RE_H5: Regex = Regex::new(r"(?i)<h5[^>]*>(.*?)</h5>").unwrap();
}

pub async fn get_body(client: &Client, url: &str) -> Result<String, Box<dyn Error>> {
    let res = client.get(url).send().await.map_err(|err| {
        eprintln!("Network Error: {}", err);
        Box::new(err) as Box<dyn Error>
    })?;

    if !res.status().is_success() {
        eprintln!("Received an error response ({}): {}", res.status(), res.text().await?);
        return Err("Error response received".into());
    }

    let body = res.text().await.map_err(|err| {
        eprintln!("Response Error: {}", err);
        Box::new(err) as Box<dyn Error>
    })?;

    let cleaned_body = RE_H5.replace_all(&body, ""); // H5-Elements are removed from the body

    Ok(cleaned_body.into_owned())
}

pub fn extract_direct_link(body: &String) -> Result<Option<String>, Box<dyn Error>> {
    let document = Html::parse_document(body);
    let selector = Selector::parse(ATTENDANCE_LINK_SELECTOR).unwrap();
    let element = document.select(&selector).next();

    if let Some(element) = element {
        let mut direct_link = element.value().attr("href")
            .ok_or("direct link not found")?.to_string();

        direct_link = format!("{}&view=5", direct_link); // append &view=5 to the link

        Ok(Some(direct_link))
    } else {
        Ok(None)
    }
}
