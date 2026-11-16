use std::path::PathBuf;

use pob_engine::{Engine, EngineConfig};
use serde_json::{json, Value};

// The text the game puts on the clipboard for Ctrl+C on an item.
const GAME_RING: &str = "Item Class: Rings\nRarity: Rare\nOverlay Loop\nIron Ring\n--------\nRequirements:\nLevel: 20\n--------\nItem Level: 80\n--------\n+70 to maximum Life\n20% increased Cast Speed\n";

fn state(engine: &Engine) -> Value {
    let xml = engine.call("save_build_xml", &Value::Null).unwrap();
    json!({
        "items": engine.call("get_items", &Value::Null).unwrap(),
        "xml": xml,
        "undo": engine.eval("local t = launch.main.modes.BUILD.itemsTab; return { undo = #t.undo, redo = #t.redo }").unwrap(),
    })
}

#[test]
fn game_item_text_is_scored_per_slot_without_changing_the_build() {
    let root = std::env::var_os("POB_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/pob"));
    if !root.join("Launch.lua").is_file() {
        eprintln!("skipping: run pob-sync or set POB_ROOT");
        return;
    }
    let user_dir = std::env::temp_dir().join(format!("pob-item-evaluate-{}", std::process::id()));
    let engine = Engine::boot(EngineConfig { pob_root: root, user_dir: user_dir.clone(), cache_dir: None, data_dir: None }).unwrap();
    engine.call("new_build", &json!({ "name": "Evaluate test" })).unwrap();
    engine.call("equip_item_raw", &json!({ "text": "Rarity: Normal\nIron Ring", "slot": "Ring 1" })).unwrap();
    let before = state(&engine);

    let r = engine.call("item_evaluate", &json!({ "raw": GAME_RING })).unwrap();
    assert_eq!(r["item"]["name"], "Overlay Loop");
    assert_eq!(r["item"]["baseName"], "Iron Ring");
    assert_eq!(r["item"]["levelRequired"], 20);
    assert_eq!(r["tooLowLevel"], true, "a level 1 character cannot wear a level 20 ring");
    let slots = r["slots"].as_array().unwrap();
    let names: Vec<_> = slots.iter().map(|s| s["slot"].as_str().unwrap()).collect();
    assert_eq!(names, ["Ring 1", "Ring 2"]);
    assert_eq!(slots[0]["replacing"], "Iron Ring");
    assert_eq!(slots[1]["replacing"], Value::Null);
    for s in slots {
        assert!(s["delta"]["Life"].as_f64().unwrap() >= 70.0, "{} Life delta {}", s["slot"], s["delta"]);
    }
    assert_eq!(state(&engine), before, "evaluation must leave the build and undo history unchanged");

    // The numbers are the ones the build has once the item is really equipped.
    engine.call("equip_item_raw", &json!({ "text": GAME_RING, "slot": "Ring 2" })).unwrap();
    let life = engine.call("get_stats", &json!({ "fields": ["Life"] })).unwrap()["stats"]["Life"].as_f64().unwrap();
    assert!((life - slots[1]["after"]["Life"].as_f64().unwrap()).abs() < 0.01, "equipped Life {life} vs {}", slots[1]["after"]["Life"]);

    let err = engine.call("item_evaluate", &json!({ "raw": "hello" })).unwrap_err().to_string();
    assert!(err.contains("unrecognised item text"), "{err}");
    let _ = std::fs::remove_dir_all(user_dir);
}
