use chrono::{Datelike, FixedOffset, Timelike, Utc};
use dotenv::dotenv;
use std::env::var;
use std::time::Duration;
use thirtyfour::prelude::*;
use thirtyfour::{By, ChromeCapabilities, WebDriver};
use tokio::time::sleep;

const PATH_CAM_0: &str = "/data/cam0/";
const PATH_CAM_1: &str = "/data/cam1/";

#[tokio::main]
async fn main() {
    dotenv().ok();

    let user = var("AIDOT_USER").expect("Missing AIDOT_USER in environment");
    let pass = var("AIDOT_PASSWORD").expect("Missing AIDOT_PASSWORD in environment");
    let url_cam_0 = var("URL_CAM_0").expect("Missing URL_CAM_0 in environment");
    let url_cam_1 = var("URL_CAM_1").expect("Missing URL_CAM_1 in environment");

    let mut caps = DesiredCapabilities::chrome();

    caps.add_arg("--no-sandbox").unwrap();
    caps.add_arg("--disable-setuid-sandbox").unwrap();
    caps.add_arg("--use-fake-ui-for-media-stream").unwrap();
    caps.add_arg("--use-fake-device-for-media-stream").unwrap();
    caps.add_arg("--allow-file-access-from-files").unwrap();
    caps.add_arg("--autoplay-policy=no-user-gesture-required").unwrap();
    caps.add_arg("--disable-features=IsolateOrigins,site-per-process").unwrap();

    let caps_clone = caps.clone();
    let user_clone = user.clone();
    let pass_clone = pass.clone();

    let t0 = tokio::spawn(camera_task(
        0, caps_clone, user_clone, pass_clone, url_cam_0, PATH_CAM_0,
    ));

    let t1 = tokio::spawn(camera_task(
        1, caps, user, pass, url_cam_1, PATH_CAM_1,
    ));

    let _ = tokio::join!(t0, t1);
}

async fn camera_task(
    id: u8,
    caps: ChromeCapabilities,
    user: String,
    pass: String,
    cam_url: String,
    path: &str,
) {
    println!("Spawn task {}", id);
    std::fs::create_dir_all(path).unwrap();
    let webdriver_url = var("WEBDRIVER_URL").unwrap_or_else(|_| "http://selenium:4444".to_string());
    let driver = WebDriver::new(&webdriver_url, caps)
        .await
        .unwrap();

    driver_sign_in(&driver, &user, &pass).await;
    sleep(Duration::from_secs(5)).await;

    driver.goto(&cam_url).await.unwrap();

    // Wait for initial video load
    let mut retries = 0;
    while wait_for_video(&driver).await.is_none() {
        sleep(Duration::from_secs(1)).await;
        retries += 1;
        if retries >= 30 {
            driver.refresh().await.unwrap();
            retries = 0;
        }
    }

    loop {
        take_picture(&driver, path).await;
        println!("TICK cam{} {}", id, get_timestamp());
        sleep(Duration::from_secs(10)).await;
    }
}

fn get_timestamp() -> String {
    let offset = FixedOffset::west_opt(3 * 3600).expect("Offset inválido");
    let datetime = Utc::now().with_timezone(&offset);
    format!(
        "{:04}{:02}{:02}{:02}{:02}{:02}",
        datetime.year(),
        datetime.month(),
        datetime.day(),
        datetime.hour(),
        datetime.minute(),
        datetime.second()
    )
}

async fn driver_sign_in(driver: &WebDriver, user: &str, pass: &str) {
    driver.goto("https://app.aidot.com/SignIn").await.unwrap();
    let current_url = driver.current_url().await.unwrap().to_string();
    if current_url.contains("/SignIn") {
        tokio::time::sleep(Duration::from_secs(3)).await;

        let username = driver
            .query(By::Css("input[placeholder='User Name']"))
            .first()
            .await
            .unwrap();
        username.send_keys(user).await.unwrap();

        let password = driver
            .query(By::Css("input[placeholder='Password']"))
            .first()
            .await
            .unwrap();
        password.send_keys(pass).await.unwrap();
        tokio::time::sleep(Duration::from_millis(500)).await;

        let submit_btn = driver
            .query(By::Css("button[type='button'].MuiButton-root"))
            .first()
            .await
            .unwrap();
        submit_btn.click().await.unwrap();
    }
}

async fn wait_for_video(driver: &WebDriver) -> Option<thirtyfour::WebElement> {
    let script = r#"
        let video = document.querySelector('video');
        return video && video.videoWidth > 0 && video.videoHeight > 0;
    "#;

    match driver.execute(script, vec![]).await {
        Ok(result) => {
            if result.json().as_bool().unwrap_or(false) {
                let video_elem = driver.query(By::Css("video")).first().await.unwrap();
                return Some(video_elem);
            }
        }
        Err(e) => {
            println!("Error al ejecutar script: {:?}", e);
        }
    }

    None
}

async fn take_picture(driver: &WebDriver, path: &str) {
    match wait_for_video(driver).await {
        Some(video_elem) => {
            let screenshot = video_elem.screenshot_as_png().await.unwrap();
            let filename = format!("{}{}.png", path, get_timestamp());
            std::fs::write(format!("{}now.png", path), &screenshot).unwrap();
            std::fs::write(&filename, &screenshot).unwrap();
            println!("Captura guardada: {}", filename);
        }
        None => {
            println!("Video no disponible, refrescando...");
            driver.refresh().await.unwrap();
            while wait_for_video(driver).await.is_none() {
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}
