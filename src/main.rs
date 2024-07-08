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

    // Create a window for displaying images
    let mut window = Window::new(
        "WebSocket Image Viewer",
        1920,
        1080,
        WindowOptions::default(),
    )?;

    let mut current_stream_key = live_views.keys().next().unwrap().to_string();
    let mut current_ws_stream: Option<WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>> = None;

    // Connect to the initial stream
    if let Some(ws_url) = live_views[&current_stream_key].as_str() {
        let (ws_stream, _) = connect_async(ws_url).await?;
        current_ws_stream = Some(ws_stream);
    }

    loop {
        // Check for key presses
        let keys = window.get_keys_pressed(minifb::KeyRepeat::No);
        for key in keys {
            let new_key = match key {
                Key::Key1 => live_views.keys().nth(0),
                Key::Key2 => live_views.keys().nth(1),
                Key::Key3 => live_views.keys().nth(2),
                Key::Key4 => live_views.keys().nth(3),
                Key::Key5 => live_views.keys().nth(4),
                Key::Escape => return Ok(()),
                _ => None,
            };

            if let Some(new_key) = new_key {
                if new_key != &current_stream_key {
                    // Disconnect from the current stream
                    if let Some(mut stream) = current_ws_stream.take() {
                        tokio::spawn(async move {
                            let _ = stream.close(None).await;
                        });
                    }

                    // Connect to the new stream
                    current_stream_key = new_key.to_string();
                    if let Some(ws_url) = live_views[&current_stream_key].as_str() {
                        let (ws_stream, _) = connect_async(ws_url).await?;
                        current_ws_stream = Some(ws_stream);
                    }
                }
            }
        }

        // Read from the current WebSocket
        if let Some(ws_stream) = &mut current_ws_stream {
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