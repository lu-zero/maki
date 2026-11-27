//! Opening a URL in the browser on the machine maki runs on.

// webbrowser runs these in the foreground on maki's terminal. Its list is
// private, so keep the same names here.
const TEXT_BROWSERS: [&str; 9] = [
    "lynx", "links", "links2", "elinks", "w3m", "eww", "netrik", "retawq", "curl",
];

/// Opens `url` in the user's browser, honoring `$BROWSER` where the platform
/// supports it. The launcher runs on the current thread, so callers on an
/// async thread use the blocking pool. Callers that own the terminal tear it
/// down when [`browser_env_uses_tty`] is true.
pub fn open_url(url: &str) -> Result<(), String> {
    webbrowser::open(url).map_err(|e| e.to_string())
}

/// Whether `$BROWSER` picks a browser that draws on the terminal. Like
/// webbrowser, only the first entry found on PATH decides.
pub fn browser_env_uses_tty() -> bool {
    std::env::var("BROWSER").is_ok_and(|browser| tty_browser(&browser))
}

fn tty_browser(browser: &str) -> bool {
    tty_browser_with(browser, program_on_path)
}

fn tty_browser_with(browser: &str, on_path: impl Fn(&str) -> bool) -> bool {
    browser
        .split(':')
        .filter_map(|entry| entry.split_whitespace().next())
        .find(|program| on_path(program))
        .is_some_and(|program| {
            let name = program.rsplit(['/', '\\']).next().unwrap_or(program);
            TEXT_BROWSERS.contains(&name)
        })
}

fn program_on_path(program: &str) -> bool {
    if program.contains('/') || program.contains('\\') {
        return std::path::Path::new(program).is_file();
    }
    std::env::var_os("PATH")
        .is_some_and(|dirs| std::env::split_paths(&dirs).any(|dir| dir.join(program).is_file()))
}

#[cfg(test)]
mod tests {
    use super::tty_browser_with;
    use test_case::test_case;

    #[test_case("lynx", true ; "plain")]
    #[test_case("/usr/bin/w3m -child", true ; "path_with_args")]
    #[test_case("helper.sh:elinks", true ; "colon_list")]
    #[test_case("firefox:lynx", false ; "gui_first_with_tty_fallback")]
    #[test_case("firefox --new-window %s", false ; "gui")]
    #[test_case("", false ; "empty")]
    fn a_terminal_browser_in_browser_is_detected(browser: &str, expected: bool) {
        assert_eq!(tty_browser_with(browser, |p| p != "helper.sh"), expected);
    }
}
