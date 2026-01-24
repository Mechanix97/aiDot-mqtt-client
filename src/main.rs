use chrono::{Datelike, FixedOffset, Timelike, Utc};
use dotenv::dotenv;
use log::error;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use std::env::var;
use std::time::Duration;
use thirtyfour::prelude::*;
use thirtyfour::{By, ChromeCapabilities, WebDriver};
use tokio::sync::broadcast;
use tokio::time::sleep;

const TOPIC_CAM_0: &str = "aidot/get/cam0";
const TOPIC_CAM_1: &str = "aidot/get/cam1";

const PATH_CAM_0: &str = "/data/cam0/";
const PATH_CAM_1: &str = "/data/cam1/";

#[tokio::main]
async fn main() {
    dotenv().ok();
    let mqtt_host = var("MQTT_HOST").unwrap_or_else(|_| "mqtt".to_string());
    let mut mqttoptions = MqttOptions::new("test-client2", &mqtt_host, 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    // Create the client and event loop
    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    if let Err(e) = client.subscribe(TOPIC_CAM_0, QoS::AtLeastOnce).await {
        error!("Failed to subscribe: {:?}", e);
        return;
    }

    if let Err(e) = client.subscribe(TOPIC_CAM_1, QoS::AtLeastOnce).await {
        error!("Failed to subscribe: {:?}", e);
        return;
    }

    let user = var("AIDOT_USER").expect("Missing AIDOT_USER in environment");
    let pass = var("AIDOT_PASSWORD").expect("Missing AIDOT_PASSWORD in environment");
    let url_cam_0 = var("URL_CAM_0").expect("Missing URL_CAM_0 in environment");
    let url_cam_1 = var("URL_CAM_1").expect("Missing URL_CAM_1 in environment");

    let mut caps = DesiredCapabilities::chrome();

    /* chrome args */
    // caps.add_arg("--headless").unwrap();
    // caps.add_arg("--disable-setuid-sandbox").unwrap();
    // caps.add_arg("--use-fake-ui-for-media-stream")
    //     .unwrap();
    caps.add_arg("--use-fake-device-for-media-stream").unwrap();
    caps.add_arg("--allow-file-access-from-files").unwrap();
    // caps.add_arg("--allow-insecure-localhost").unwrap();
    // caps.add_arg("--no-sandbox").unwrap();
    // caps.add_arg("--disable-web-security").unwrap();
    // caps.add_arg("--disable-features=IsolateOrigins,site-per-process")
    //     .unwrap();

    let (tx, _) = broadcast::channel::<(String, Vec<u8>)>(32);

    // Task de recepción MQTT
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    //  println!("Connected to broker!");
                }
                Ok(Event::Incoming(Incoming::Publish(p))) => {
                    let _ = tx_clone.send((p.topic.clone(), p.payload.to_vec()));
                }
                Ok(_) => {}
                Err(_e) => {}
            }
        }
    });

    let caps_clone = caps.clone();
    let user_clone = user.clone();
    let pass_clone = pass.clone();
    let rx_0 = tx.subscribe();
    tokio::spawn(camera_task(
        0, caps_clone, user_clone, pass_clone, url_cam_0, TOPIC_CAM_0, PATH_CAM_0, rx_0,
    ));

    let rx_1 = tx.subscribe();
    tokio::spawn(camera_task(
        1, caps, user, pass, url_cam_1, TOPIC_CAM_1, PATH_CAM_1, rx_1,
    ));

    loop {
        client
            .publish(TOPIC_CAM_0, QoS::AtLeastOnce, false, "")
            .await
            .unwrap();
        client
            .publish(TOPIC_CAM_1, QoS::AtLeastOnce, false, "")
            .await
            .unwrap();

        println!("TICK {}", get_timestamp());

        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}

async fn camera_task(
    id: u8,
    caps: ChromeCapabilities,
    user: String,
    pass: String,
    cam_url: String,
    topic: &str,
    path: &str,
    mut rx: broadcast::Receiver<(String, Vec<u8>)>,
) {
    println!("Spawn task {}", id);
    std::fs::create_dir_all(path).unwrap();
    let webdriver_url = var("WEBDRIVER_URL").unwrap_or_else(|_| "http://selenium:4444".to_string());
    let driver = WebDriver::new(&webdriver_url, caps)
        .await
        .unwrap();

    driver_sign_in(&driver, &user, &pass).await;
    tokio::time::sleep(Duration::from_secs(5)).await;

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
        match rx.recv().await {
            Ok((msg_topic, payload)) => {
                if msg_topic == topic {
                    println!("Task {}: {:?}", id, String::from_utf8_lossy(&payload));
                    take_picture(&driver, path).await;
                }
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                println!("Task {}: skipped {} messages", id, n);
            }
            Err(broadcast::error::RecvError::Closed) => {
                println!("Task {}: channel closed", id);
                break;
            }
        }
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
        // Rellenar campos de login
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

// Función para esperar a que el video tenga dimensiones válidas y devolver el elemento
async fn wait_for_video(driver: &WebDriver) -> Option<thirtyfour::WebElement> {
    let script = r#"
        let video = document.querySelector('video');
        return video && video.videoWidth > 0 && video.videoHeight > 0;
    "#;

    match driver.execute(script, vec![]).await {
        Ok(result) => {
            if result.json().as_bool().unwrap_or(false) {
                // Obtener el elemento <video> cuando esté listo
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
            // Tomar captura de pantalla solo del elemento <video>
            println!("📸 Tomando captura del elemento <video>...");
            let screenshot = video_elem.screenshot_as_png().await.unwrap();
            let filename = format!("{}{}.png", path, get_timestamp());
            std::fs::write(format!("{}now.png", path), &screenshot).unwrap();
            std::fs::write(&filename, &screenshot).unwrap();
            println!("✅ Captura guardada como {}", filename);
        }
        None => {
            println!("❌ No se pudo cargar el video con dimensiones válidas");
            driver.refresh().await.unwrap();
            while wait_for_video(driver).await.is_none() {
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}
