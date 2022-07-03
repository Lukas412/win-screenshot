use regex::Regex;
use win_screenshot::addon::*;
use win_screenshot::capture::*;

fn main() {
    // capture entire screen
    /*capture_display().unwrap().save("screenshot.jpg").unwrap();

    // capture window by known id
    capture_window(11996706)
        .unwrap()
        .save("screenshot.jpg")
        .unwrap();

    // capture window if you know the exact name
    capture_window(find_window("BlueStacks").unwrap())
        .unwrap()
        .save("screenshot.jpg")
        .unwrap();

    // if you don't know the exact name, try to find it
    let re = Regex::new(r"Firefox").unwrap();
    capture_window(
        window_list()
            .unwrap()
            .iter()
            .find(|i| re.is_match(&i.window_name))
            .unwrap()
            .hwnd,
    )
    .unwrap()
    .save("screenshot.jpg")
    .unwrap();*/

    dbg!(window_list());
}
