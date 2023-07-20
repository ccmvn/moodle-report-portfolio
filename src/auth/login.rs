use lazy_static::lazy_static;
use log::info;
use reqwest::Client;
use scraper::{Html, Selector};
use crate::{CONFIG};
use crate::auth::client::get_body;
use crate::common::tables::LoginForm;

const LOGIN_URL: &str = "https://lernplattform.gfn.de/login/index.php";

lazy_static! {
    static ref LOGINTOKEN_SELECTOR: Selector = Selector::parse(r#"input[name="logintoken"]"#).unwrap();
    static ref ERROR_SELECTOR: Selector = Selector::parse(r#"div.alert.alert-danger"#).unwrap();
}

fn extract_logintoken(body: &String) -> Result<String, Box<dyn std::error::Error>> {
    let document = Html::parse_document(body);
    let logintoken = document.select(&LOGINTOKEN_SELECTOR).next()
        .ok_or("no logintoken input found")?
        .value().attr("value")
        .ok_or("logintoken value not found")?.to_string();

    Ok(logintoken)
}

pub async fn login(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    let body = get_body(client, LOGIN_URL).await?;
    let logintoken = extract_logintoken(&body)?;

    let form = LoginForm {
        username: CONFIG.get_user_name().to_owned(),
        password: CONFIG.get_password().to_owned(),
        logintoken,
    };

    post_login(client, form).await
}

async fn post_login(client: &Client, form: LoginForm) -> Result<(), Box<dyn std::error::Error>> {
    let res = client.post(LOGIN_URL)
        .form(&form)
        .send()
        .await?;

    let status = res.status();
    let body = res.text().await?;

    if status.is_success() {
        let document = Html::parse_document(&body);

        if document.select(&ERROR_SELECTOR).next().is_some() {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Invalid login details")));
        }
    } else {
        info!("Login failed!");
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Login request failed")));
    }

    Ok(())
}
