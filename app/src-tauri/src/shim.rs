/// Script injected into every page loaded by the WebView (downloads,
/// external links, clipboard, update banner).
pub const SHIM_JS: &str = include_str!("shim.js");
