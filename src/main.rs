use std::collections::HashMap;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, WebSocketStream};
use futures_util::StreamExt;
use serde_json::Value;
use image::RgbImage;
use minifb::{Key, Window, WindowOptions};
use reqwest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Fetch the JSON data
    let client = reqwest::Client::new();
    let resp = client.get("http://10.3.61.100:3887/system")
        .send()
        .await?
        .json::<Value>()
        .await?;

    // Extract live_views
    let live_views = resp["live_views"].as_object()
        .expect("live_views should be an object");

    let mut ws_streams: HashMap<String, WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>> = HashMap::new();

    // Connect to all WebSockets
    for (key, value) in live_views {
        // if value
        let ws_url = value.as_str().unwrap();
        let (ws_stream, _) = connect_async(ws_url).await?;
        ws_streams.insert(key.clone(), ws_stream);
    }

    // Create a window for displaying images
    let mut window = Window::new(
        "WebSocket Image Viewer",
        1920,
        1080,
        WindowOptions::default(),
    )?;

    let mut current_stream_key = live_views.keys().next().unwrap().to_string();

    loop {
        // Check for key presses
        let keys = window.get_keys_pressed(minifb::KeyRepeat::No);
        for key in keys {
            match key {
                Key::Key1 => current_stream_key = live_views.keys().nth(0).unwrap().to_string(),
                Key::Key2 => current_stream_key = live_views.keys().nth(1).unwrap().to_string(),
                Key::Key3 => current_stream_key = live_views.keys().nth(2).unwrap().to_string(),
                Key::Key4 => current_stream_key = live_views.keys().nth(3).unwrap().to_string(),
                Key::Key5 => current_stream_key = live_views.keys().nth(4).unwrap().to_string(),
                Key::Escape => return Ok(()),
                _ => {}
            }
        }

        // Read from the current WebSocket
        if let Some(ws_stream) = ws_streams.get_mut(&current_stream_key) {
            if let Some(message) = ws_stream.next().await {
                let message = message?;
                if message.is_binary() {
                    // Decode JPEG
                    let img = image::load_from_memory(&message.into_data())?;
                    let rgb_img: RgbImage = img.to_rgb8();

                    // Convert to u32 buffer
                    let buffer: Vec<u32> = rgb_img.pixels()
                        .map(|p| {
                            let r = p[0] as u32;
                            let g = p[1] as u32;
                            let b = p[2] as u32;
                            (r << 16) | (g << 8) | b
                        })
                        .collect();

                    // Display the image
                    window.update_with_buffer(
                        &buffer,
                        rgb_img.width() as usize,
                        rgb_img.height() as usize,
                    )?;
                }
            }
        }

        // Break the loop if the window is closed
        if !window.is_open() {
            break;
        }
    }

    Ok(())
}