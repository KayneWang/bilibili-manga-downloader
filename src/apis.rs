use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    vec,
};

use bytes::Bytes;
use futures::future::join_all;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use urlencoding::encode;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

use crate::utils::{get_reqwest_headers, get_safe_filename};

#[derive(Deserialize, Debug)]
struct UserInfoResponse {
    code: i32,
    data: UserInfoBody,
}

#[derive(Deserialize, Debug)]
struct UserInfoBody {
    #[serde(rename = "isLogin")]
    pub is_login: bool,
}

pub async fn get_userinfo(cookie: &str) -> bool {
    let referer_url = "https://manga.bilibili.com/";
    let headers = get_reqwest_headers(referer_url, cookie);

    let client = reqwest::Client::new();
    let res = client
        .get("https://api.bilibili.com/x/web-interface/nav")
        .headers(headers)
        .send()
        .await;

    if res.is_err() {
        return false;
    }

    let res = res.unwrap();

    let resp_body = res.json::<UserInfoResponse>().await.unwrap();

    if resp_body.code != 0 {
        return false;
    }

    resp_body.data.is_login
}

#[derive(Deserialize, Debug)]
struct CommonResponse<T> {
    code: u8,
    data: T,
    msg: String,
}

#[derive(Deserialize, Debug)]
struct SearchMangaResponse {
    list: Vec<SearchMangaItem>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SearchMangaItem {
    pub id: u32,
    #[serde(rename = "real_title")]
    pub title: String,
    #[serde(rename = "type")]
    pub manga_type: u8,
}

pub async fn search_manga(
    name: String,
) -> Result<Vec<SearchMangaItem>, Box<dyn std::error::Error>> {
    let base_url = "https://manga.bilibili.com/twirp/comic.v1.Comic/Search?device=pc&platform=web";
    let referer_url = format!(
        "https://manga.bilibili.com/search?from=manga_homepage&keyword={}",
        encode(&name).to_string()
    );
    let headers = get_reqwest_headers(&referer_url, "");

    let mut request_body = HashMap::new();
    request_body.insert("key_word", name);
    request_body.insert("page_num", "1".to_string());
    request_body.insert("page_size", "3".to_string());

    let client = reqwest::Client::new();
    let res = client
        .post(base_url)
        .json(&request_body)
        .headers(headers)
        .send()
        .await?;

    if res.status() != reqwest::StatusCode::OK {
        println!("请求失败: {:?}", res.status());
        return Err("请求失败".into());
    }
    let resp_body = res.json::<CommonResponse<SearchMangaResponse>>().await?;

    if resp_body.code != 0 {
        println!("请求失败: {:?}", resp_body.msg);
        return Err(resp_body.msg.into());
    }

    // 过滤 Vomic 类型的漫画.
    let comic_list = resp_body
        .data
        .list
        .iter()
        .filter(|&item| item.manga_type == 0)
        .cloned()
        .collect();

    Ok(comic_list)
}

#[derive(Deserialize, Debug)]
pub struct MangaDetailResponse {
    pub ep_list: Vec<Episode>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Episode {
    pub id: u32,
    pub title: String,
    pub ord: f32,
}

pub async fn get_manga_detail(
    manga_id: &u32,
) -> Result<MangaDetailResponse, Box<dyn std::error::Error>> {
    let base_url =
        "https://manga.bilibili.com/twirp/comic.v1.Comic/ComicDetail?device=pc&platform=web";
    let referer_url = format!(
        "https://manga.bilibili.com/detail/mc{}?from=manga_search",
        &manga_id
    );
    let headers = get_reqwest_headers(&referer_url, "");

    let mut request_body = HashMap::new();
    request_body.insert("comic_id", manga_id.to_string());

    let client = reqwest::Client::new();
    let res = client
        .post(base_url)
        .json(&request_body)
        .headers(headers)
        .send()
        .await
        .unwrap();

    if res.status() != reqwest::StatusCode::OK {
        println!("请求失败: {:?}", res.status());
        return Err("请求失败".into());
    }
    let resp_body = res.json::<CommonResponse<MangaDetailResponse>>().await?;

    if resp_body.code != 0 {
        println!("请求失败: {:?}", resp_body.msg);
        return Err(resp_body.msg.into());
    }

    Ok(resp_body.data)
}

#[derive(Deserialize, Debug)]
struct ImageIndexResponse {
    images: Vec<ImageData>,
}

#[derive(Deserialize, Debug)]
struct ImageData {
    path: String,
}

#[derive(Deserialize, Debug)]
struct ImageTokenResponse {
    token: String,
    url: String,
}

async fn get_image_urls(
    manga_id: u32,
    episode_id: u32,
    cookie: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // 获取图片 url.
    let base_url =
        "https://manga.bilibili.com/twirp/comic.v1.Comic/GetImageIndex?device=pc&platform=web";
    let refer_url = format!(
        "https://manga.bilibili.com/mc{}/{}?from=manga_detail",
        manga_id, episode_id
    );
    let headers = get_reqwest_headers(&refer_url, cookie);

    let mut request_body = HashMap::new();
    request_body.insert("ep_id", episode_id.to_string());

    let client = reqwest::Client::new();
    let res = client
        .post(base_url)
        .json(&request_body)
        .headers(headers.clone())
        .send()
        .await?;

    if res.status() != reqwest::StatusCode::OK {
        println!("ImageIndex 请求失败: {:?}", res.status());
        return Err("ImageIndex 请求失败".into());
    }
    let resp_body = res.json::<CommonResponse<ImageIndexResponse>>().await?;

    if resp_body.code != 0 {
        println!("ImageIndex 请求失败: {:?}", resp_body.msg);
        return Err(resp_body.msg.into());
    }

    let image_urls: Vec<String> = resp_body
        .data
        .images
        .iter()
        .map(|image| image.path.clone())
        .collect();

    // 获取图片 token.
    let base_url =
        "https://manga.bilibili.com/twirp/comic.v1.Comic/ImageToken?device=pc&platform=web";
    let urls_str = serde_json::to_string(&image_urls).unwrap();
    let mut request_body = HashMap::new();
    request_body.insert("urls", urls_str);

    let res = client
        .post(base_url)
        .json(&request_body)
        .headers(headers)
        .send()
        .await?;
    if res.status() != reqwest::StatusCode::OK {
        println!("ImageToken 请求失败: {:?}", res.status());
        return Err("ImageToken 请求失败".into());
    }
    if resp_body.code != 0 {
        println!("ImageToken 请求失败: {:?}", resp_body.msg);
        return Err(resp_body.msg.into());
    }

    let resp_body = res
        .json::<CommonResponse<Vec<ImageTokenResponse>>>()
        .await?;
    let image_urls: Vec<String> = resp_body
        .data
        .iter()
        .map(|item| format!("{}?token={}", item.url, item.token))
        .collect();

    Ok(image_urls)
}

async fn download_image(image_urls: Vec<String>, title: &str) -> Result<Vec<Bytes>, Box<dyn std::error::Error>> {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(5));
    let mut handles = vec![];
    let progress_bar = ProgressBar::new(image_urls.len() as u64);
    progress_bar.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
        .expect("Failed to set bar template")
        .progress_chars("#>-"));

    for url in image_urls {
        let permit = semaphore.clone().acquire_owned().await?;
        let pb = progress_bar.clone();
        let handle = tokio::spawn(async move {
            let resp = reqwest::get(&url).await;
            if let Err(e) = resp {
                drop(permit);
                return Err(e);
            }
            let bytes = resp.unwrap().bytes().await?;
            drop(permit);
            pb.inc(1);
            Ok(bytes)
        });
        handles.push(handle);
    }

    let tasks = join_all(handles).await;
    let mut image_bytes = vec![];

    for task in tasks {
        let bytes = task??;
        image_bytes.push(bytes);
    }
    progress_bar.finish_with_message(format!("{} 下载完成", title));
    Ok(image_bytes)
}

async fn create_zip(
    image_bytes: Vec<Bytes>,
    dest_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(dest_path).unwrap();
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for (index, bytes) in image_bytes.iter().enumerate() {
        let file_name = format!("{}.jpg", index);
        zip.start_file(file_name, options).unwrap();
        zip.write_all(&bytes).unwrap();
    }
    zip.finish()?;
    Ok(())
}

pub async fn do_download_tasks(
    manga_id: u32,
    episodes: Vec<Episode>,
    cookie: &str,
    dest_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    for episode in episodes {
        let image_urls = get_image_urls(manga_id, episode.id, cookie).await?;
        let dest_path = PathBuf::from(dest_path).join(format!(
            "[{}]{}.zip",
            episode.ord,
            get_safe_filename(&episode.title)
        ));

        match download_image(image_urls, &episode.title).await {
            Ok(image_bytes) => {
                if let Err(e) = create_zip(image_bytes, &dest_path).await {
                    println!("{} 下载失败: {}", episode.title, e);
                    continue;
                }
            }
            Err(e) => {
                println!("{} 下载失败: {}", episode.title, e);
                continue;
            }
        }
    }

    Ok(())
}
