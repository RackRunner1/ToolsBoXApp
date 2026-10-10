/// Script injecté dans chaque page chargée par le WebView (téléchargements,
/// liens externes, presse-papier, bandeau de mise à jour).
pub const SHIM_JS: &str = include_str!("shim.js");
