use rand::Rng;
use reqwest::header::{self, HeaderMap};

fn get_random_ua() -> String {
    let uas = vec![
        // Chrome
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36",
        // Firefox
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:127.0) Gecko/20100101 Firefox/127.0",
        // Safari
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4.1 Safari/605.1.15",
        // Edge
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36 Edg/126.0.2592.81",
        // Opera
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36 OPR/111.0.0.0",
        // Vivaldi
        "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36 Vivaldi/6.8.3381.46",
        // Yandex
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 YaBrowser/24.4.5.506 Yowser/2.5 Safari/537.36",
    ];

    let mut rng = rand::thread_rng();
    let index = rng.gen_range(0..uas.len());
    uas[index].to_string()
}

pub fn get_reqwest_headers(referer_url: &str, cookie: &str) -> HeaderMap {
    let ua = get_random_ua();
    let mut headers = HeaderMap::new();
    headers.append(header::ORIGIN, "https://manga.bilibili.com".parse().unwrap());
    headers.append(header::REFERER, referer_url.parse().unwrap());
    headers.append(header::USER_AGENT, ua.parse().unwrap());

    let cookie = format!("SESSDATA={}", cookie);
    headers.append(header::COOKIE, cookie.parse().unwrap());
    headers.append(header::CONTENT_TYPE, "application/json;charset=UTF-8".parse().unwrap());

    headers
}

pub fn path_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

pub fn create_desc_dir(path: &str) {
    if path_exists(path) {
        return;
    }
    std::fs::create_dir_all(path).expect("创建文件夹失败");
}

pub fn get_safe_filename(filename: &str) -> String {
    let reg = regex::Regex::new("[\\/:*?\"<>|\\s]").unwrap();
    reg.replace_all(filename, "").to_string()
}
