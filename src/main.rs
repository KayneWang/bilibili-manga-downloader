use std::{
    collections::HashMap,
    fs,
    io::{self},
    path::{Path, PathBuf},
    time::Duration,
};

use apis::{
    do_download_tasks, get_manga_detail, get_userinfo, search_manga, Episode, SearchMangaItem,
};
use clap::Parser;
use crossterm::{
    cursor::MoveTo,
    event::{self, KeyCode, KeyEvent},
    execute,
    style::Stylize,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use dialoguer::{theme::ColorfulTheme, Input, MultiSelect, Select};
use dirs::config_dir;
use indicatif::ProgressBar;
use tokio;
use utils::{create_desc_dir, get_safe_filename, path_exists};

mod apis;
mod config;
mod utils;

async fn load_user_config(download_path: Option<String>) -> config::Config {
    let mut config_path = config_dir().unwrap_or_else(|| PathBuf::from("."));
    config_path.push("bili_manga_downloader");
    fs::create_dir_all(&config_path).expect("创建配置文件夹失败");
    config_path.push("config.json");

    let mut config = config::Config::load(&config_path);

    if download_path.is_some() {
        config.download_path = download_path.unwrap();
    }

    loop {
        // 校验 cookie
        let is_valid_cookie = get_userinfo(&config.cookie).await;
        if !is_valid_cookie {
            let cookie: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Cookie 无效, 请输入 Bilibili Cookie(SESSDATA 里):")
                .interact_text()
                .unwrap();

            config.cookie = cookie;
        }
        break;
    }

    loop {
        // 校验下载路径
        let is_valid_download_path = path_exists(&config.download_path);
        if !is_valid_download_path {
            let download_path: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("下载路径不存在, 请输入:")
                .interact_text()
                .unwrap();

            config.download_path = download_path;
        }
        break;
    }

    config.save(&config_path);

    config
}

/**
 * 通过用户输入的漫画名获取漫画信息.
 */
async fn get_selected_manga(manga_name: Option<String>) -> Option<SearchMangaItem> {
    let input: String;
    if manga_name.is_none() {
        input = Input::<String>::with_theme(&ColorfulTheme::default())
            .with_prompt("输入漫画名称:")
            .interact_text()
            .unwrap();
    } else {
        input = manga_name.unwrap();
    }

    let input = input.trim();
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(100));
    let search_result = search_manga(input.to_string()).await;
    pb.finish();

    if let Err(e) = search_result {
        println!("漫画搜索失败: {}", e);
        return None;
    }

    let search_result = search_result.unwrap();

    let search_selections = search_result
        .iter()
        .map(|item| item.title.clone())
        .collect::<Vec<String>>();

    if search_selections.is_empty() {
        println!("没有找到相关漫画");
        return None;
    }

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("选择一部漫画:")
        .items(&search_selections)
        .default(0)
        .interact()
        .unwrap();
    let selected_manga = &search_result[selection];

    Some(selected_manga.clone())
}

async fn get_episode_pages(manga_id: &u32, cookie: &str) -> Option<Vec<Vec<Episode>>> {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(100));
    let manga_detail = get_manga_detail(manga_id, cookie).await;
    pb.finish();

    if let Err(e) = manga_detail {
        println!("获取漫画详情失败: {}", e);
        return None;
    }

    let manga_detail = manga_detail.unwrap();

    let mut episode_pages: Vec<Vec<Episode>> = Vec::new();
    let mut current_page: Vec<Episode> = Vec::new();
    for (index, episode) in manga_detail.ep_list.iter().enumerate() {
        current_page.push(episode.clone());

        if index % 10 == 0 && index != 0 {
            episode_pages.push(current_page.clone());
            current_page.clear();
        }
    }

    if !current_page.is_empty() {
        episode_pages.push(current_page);
    }

    Some(episode_pages)
}

/**
 * 获取用户选择的章节信息.
 */
fn get_selected_episodes(episode_pages: &Vec<Vec<String>>) -> HashMap<String, Vec<usize>> {
    let mut current_page = 0;

    let mut select_episode_map: HashMap<String, Vec<usize>> = HashMap::new();

    let mut stdout = io::stdout();

    if let Err(e) = enable_raw_mode() {
        eprintln!("Failed to enable raw mode: {:?}", e);
        return select_episode_map;
    }

    loop {
        if let Err(e) = event::read() {
            eprintln!("Failed to read event: {:?}", e);
            break;
        }

        execute!(stdout, MoveTo(0, 0)).unwrap();
        execute!(stdout, Clear(ClearType::All)).unwrap();

        println!("{}", "****** 操作说明 ******".blue());
        println!("- {}", "使用上下箭头翻页".cyan());
        println!("- {}", "Enter 进入选择".cyan());
        println!("- {}", "a 全选".cyan());
        println!("- {}", "空格 确认选择".cyan());

        println!("{}", "****** 章节预览 ******".blue());
        let mut default_select = vec![];
        for (index, item) in episode_pages[current_page].iter().enumerate() {
            let selected_episodes = select_episode_map.get(&current_page.to_string());
            if selected_episodes.is_none() {
                println!("{}", item);
                default_select.push(false);
            } else {
                let selected_episodes = selected_episodes.unwrap();
                if selected_episodes.contains(&index) {
                    println!("{} {}", item, "\u{2713}".green());
                    default_select.push(true);
                } else {
                    println!("{}", item);
                    default_select.push(false);
                }
            }
        }

        let event = event::read().unwrap();
        if let event::Event::Key(KeyEvent { code, .. }) = event {
            match code {
                KeyCode::Up => {
                    if current_page > 0 {
                        current_page -= 1;
                    } else {
                        current_page = episode_pages.len() - 1;
                    }
                }
                KeyCode::Down => {
                    if current_page < episode_pages.len() - 1 {
                        current_page += 1;
                    } else {
                        current_page = 0;
                    }
                }
                KeyCode::Enter => {
                    execute!(stdout, MoveTo(0, 0)).unwrap();
                    execute!(stdout, Clear(ClearType::All)).unwrap();

                    disable_raw_mode().unwrap();

                    let selection = MultiSelect::with_theme(&ColorfulTheme::default())
                        .with_prompt("按 '空格' 选中章节 (Enter 确认选择, ESC/q 退出选择)")
                        .items(&episode_pages[current_page])
                        .defaults(&default_select)
                        .interact_opt();

                    let selection = selection.unwrap();

                    if let Some(selection) = &selection {
                        select_episode_map.insert(current_page.to_string(), selection.to_vec());
                    }

                    enable_raw_mode().unwrap();
                }
                KeyCode::Char(' ') => break,
                KeyCode::Char('a') => {
                    let mut all_select = vec![];
                    for (index, _) in episode_pages[current_page].iter().enumerate() {
                        all_select.push(index);
                    }
                    select_episode_map.insert(current_page.to_string(), all_select);
                }
                _ => {}
            }
        }
    }

    disable_raw_mode().unwrap();

    select_episode_map
}

#[derive(Parser, Debug)]
#[command(version, about, long_about=None, author)]
struct Args {
    #[arg(short, long)]
    manga_name: Option<String>,
    #[arg(short, long)]
    download_path: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let config = load_user_config(args.download_path).await;

    println!("漫画下载目录: {}", config.download_path.clone().cyan());

    let selected_manga = get_selected_manga(args.manga_name).await;

    if selected_manga.is_none() {
        return;
    }

    let selected_manga = selected_manga.unwrap();

    let episode_pages = get_episode_pages(&selected_manga.id, &config.cookie).await;

    if episode_pages.is_none() {
        return;
    }

    let episode_pages = episode_pages.unwrap();
    let episode_pages_selections = episode_pages
        .iter()
        .map(|page| {
            page.iter()
                .map(|episode| {
                    if episode.is_locked {
                        return format!(
                            "[{}]{} {}",
                            episode.ord.clone(),
                            episode.title.clone(),
                            "🔒".red()
                        );
                    }
                    format!(
                        "[{}]{} {}",
                        episode.ord.clone(),
                        episode.title.clone(),
                        "🔓".green()
                    )
                })
                .collect()
        })
        .collect::<Vec<Vec<String>>>();

    let selected_episodes = get_selected_episodes(&episode_pages_selections);

    if selected_episodes.is_empty() {
        return;
    }

    let mut download_episodes: Vec<Episode> = Vec::new();
    for (page, ep_indexes) in selected_episodes.iter() {
        let episodes = &episode_pages[page.parse::<usize>().unwrap()];
        for ep_index in ep_indexes {
            let episode = &episodes[*ep_index];
            download_episodes.push(episode.clone());
        }
    }

    if download_episodes.is_empty() {
        return;
    }

    let manga_title = get_safe_filename(&selected_manga.title);
    let dest_path = Path::new(&config.download_path).join(&manga_title);
    // 创建下载目录
    create_desc_dir(&dest_path.to_str().unwrap());

    // 获取下载失败提示信息
    let failed_message = do_download_tasks(
        selected_manga.id,
        download_episodes,
        &config.cookie,
        &dest_path,
    )
    .await;

    if failed_message.is_empty() {
        println!("{}", "所有章节下载完成".green());
        return;
    }

    for message in failed_message {
        println!("{}", message.red());
    }
}
