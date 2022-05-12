# win-screenshot
Take a screenshot from specified window or whole display on windows platform

Example
```rust
fn main() {
    // capture whole display
    capture_display().unwrap().save("screenshot.jpg").unwrap();

    // capture window by known id
    capture_window(67136).unwrap().save("screenshot.jpg").unwrap();

    // capture window by Name
    let window_name = "WindowName";

    match find_window(window_name) {
        Ok(hwnd) => capture_window(hwnd)
            .unwrap().save("screenshot.jpg").unwrap(),
        Err(_) => panic!("window {} not found!", window_name)
    }
}
```