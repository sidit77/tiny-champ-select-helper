
#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon_with_id("assets/favicon.ico", "favicon");
    res.compile().unwrap();
}

#[cfg(not(windows))]
fn main() {

}