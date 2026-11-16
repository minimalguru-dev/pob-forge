use std::path::PathBuf;

use pob_engine::{Engine, EngineConfig};
use serde_json::{json, Value};

#[test]
fn trade_search_url_builds_a_ring_query_without_the_trade_api() {
    let root = std::env::var_os("POB_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/pob"));
    if !root.join("Launch.lua").is_file() || !root.join("Data/QueryMods.lua").is_file() {
        eprintln!("skipping: run pob-sync or set POB_ROOT");
        return;
    }
    let user_dir = std::env::temp_dir().join(format!("pob-trade-search-{}", std::process::id()));
    let engine = Engine::boot(EngineConfig { pob_root: root, user_dir: user_dir.clone(), cache_dir: None, data_dir: None }).unwrap();
    engine.call("new_build", &json!({ "name": "Trade search test" })).unwrap();
    engine.call("equip_item_raw", &json!({ "text": "Rarity: Normal\nIron Ring", "slot": "Ring 1" })).unwrap();
    let xml = engine.call("save_build_xml", &Value::Null).unwrap();

    let r = engine
        .call(
            "trade_search_url",
            &json!({ "slotName": "Ring 1", "league": "Standard", "statWeights": [{ "stat": "Life", "weightMult": 1 }] }),
        )
        .unwrap();
    let url = r["url"].as_str().unwrap();
    assert!(
        url.starts_with("https://www.pathofexile.com/trade/search/Standard?q=") || url.starts_with("https://www.pathofexile.com/trade2/search/poe2/Standard?q="),
        "{url}"
    );
    assert_eq!(r["slot"], "Ring 1");
    assert_eq!(r["league"], "Standard");
    assert_eq!(r["category"], "accessory.ring");
    assert_eq!(r["statWeights"][0]["stat"], "Life");

    // The link carries exactly the query PoB generated.
    let query: Value = serde_json::from_str(r["query"].as_str().unwrap()).unwrap();
    assert_eq!(query["query"]["filters"]["type_filters"]["filters"]["category"]["option"], "accessory.ring");
    let weighted = r["weighted"].as_array().unwrap();
    assert!(!weighted.is_empty(), "a Life-weighted ring query weighs some mods: {r}");
    assert!(
        weighted.iter().any(|w| w["text"].as_str().unwrap_or_default().contains("maximum Life")),
        "maximum Life should be among the weighted mods: {weighted:?}"
    );
    let encoded = url.split("?q=").nth(1).unwrap();
    assert!(!encoded.contains('{') && !encoded.contains('"'), "the query is URL-encoded");

    let err = engine
        .call("trade_search_url", &json!({ "slotName": "Ring 1", "league": "Standard", "statWeights": [{ "stat": "NotAStat" }] }))
        .unwrap_err()
        .to_string();
    assert!(err.contains("unknown stat NotAStat"), "{err}");

    // Generating a search changes nothing in the build.
    assert_eq!(engine.call("save_build_xml", &Value::Null).unwrap(), xml);
    let _ = std::fs::remove_dir_all(user_dir);
}
