use std::path::PathBuf;

use pob_engine::{Engine, EngineConfig};
use serde_json::{json, Value};

// A rare map as the game copies it with Ctrl+C (PoE1).
const MAP: &str = "Item Class: Maps\nRarity: Rare\nTest Grove\nStrand Map\n--------\nMap Tier: 14\nItem Quantity: +70% (augmented)\n--------\nItem Level: 81\n--------\nMonster Level: 81\n--------\n+40% Monster Physical Damage Reduction\nPlayers have -12% to all maximum Resistances\nPlayers cannot Regenerate Life, Mana or Energy Shield\nMonsters reflect 18% of Physical Damage\nArea contains two Unique Bosses\n--------\nTravel to this Map by using it in a personal Map Device. Maps can only be used once.\n";

#[test]
fn map_mods_are_recognised_and_calculated_on_a_copy() {
    let root = std::env::var_os("POB_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/pob1"));
    if !root.join("Launch.lua").is_file() {
        eprintln!("skipping: run pob-sync or set POB_ROOT");
        return;
    }
    let poe2 = root.ends_with("pob");
    let user_dir = std::env::temp_dir().join(format!("pob-map-evaluate-{}", std::process::id()));
    let engine = Engine::boot(EngineConfig { pob_root: root, user_dir: user_dir.clone(), cache_dir: None, data_dir: None }).unwrap();
    engine.call("new_build", &json!({ "name": "Map test" })).unwrap();
    let group = engine.call("add_socket_group", &json!({})).unwrap();
    engine.call("add_gem", &json!({ "groupIndex": group["groupIndex"], "nameSpec": if poe2 { "Bonk" } else { "Heavy Strike" }, "level": 1 })).ok();
    let before = engine.call("save_build_xml", &Value::Null).unwrap();

    let r = engine.call("map_evaluate", &json!({ "raw": MAP })).unwrap();
    assert_eq!(r["tier"], 14);
    let names: Vec<_> = r["mods"].as_array().unwrap().iter().map(|m| m["name"].as_str().unwrap().to_string()).collect();
    assert!(names.iter().any(|n| n == "Armoured"), "recognised {names:?}");
    let unmatched: Vec<_> = r["unmatched"].as_array().unwrap().iter().map(|l| l.as_str().unwrap()).collect();
    assert!(!unmatched.iter().any(|l| l.contains("Travel to this Map") || l.contains("Item Level")), "{unmatched:?}");
    let warnings: Vec<_> = r["warnings"].as_array().unwrap().iter().map(|w| w["text"].as_str().unwrap().to_string()).collect();
    assert!(warnings.iter().any(|w| w.starts_with("No regeneration")), "{warnings:?}");
    if !poe2 {
        assert_eq!(r["calculated"], true);
        assert_eq!(r["tierBand"], "HIGH");
        assert!(names.iter().any(|n| n == "of Exposure"), "recognised {names:?}");
        // High tier of Exposure is 9-12%; a copy takes the top of the roll.
        assert_eq!(r["delta"]["FireResistMax"].as_f64(), Some(-12.0), "{}", r["delta"]);
    }
    assert_eq!(engine.call("save_build_xml", &Value::Null).unwrap(), before, "the live build must not change");
    let _ = std::fs::remove_dir_all(user_dir);
}
