use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use reqwest_cookie_store::CookieStoreMutex;

const COOKIE_STORE_PATH: &str = "./cookies.json";

pub fn load_cookie_store() -> Arc<CookieStoreMutex> {
    let cookie_store = {
        if let Ok(file) = File::open(COOKIE_STORE_PATH).map(BufReader::new) {
            reqwest_cookie_store::CookieStore::load_json(file).unwrap()
        } else {
            reqwest_cookie_store::CookieStore::new(None)
        }
    };

    CookieStoreMutex::new(cookie_store).into()
}

pub fn save_cookies(cookie_store: Arc<CookieStoreMutex>) -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = std::fs::File::create(COOKIE_STORE_PATH)
        .map(std::io::BufWriter::new)
        .unwrap();

    let store = cookie_store.lock().unwrap();
    store.save_json(&mut writer).unwrap();

    Ok(())
}
