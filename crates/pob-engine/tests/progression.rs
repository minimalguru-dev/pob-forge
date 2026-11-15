use pob_engine::{Engine, EngineConfig};
use serde_json::{json, Value};
use std::path::PathBuf;

#[test]
fn progression_lua_compiles() {
    let lua = mlua::Lua::new();
    let _: mlua::Function = lua.load(include_str!("../lua/progression.lua")).eval().unwrap();
}

#[test]
#[ignore = "requires synced PoE1 data; run with POB_REDUX_POB_ROOT and --ignored"]
fn progression_uses_pob_without_changing_live_build() {
    let root = PathBuf::from(std::env::var("POB_REDUX_POB_ROOT").expect("set POB_REDUX_POB_ROOT"));
    let root = if root.is_absolute() { root } else { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join(root) };
    let user_dir = std::env::temp_dir().join(format!("redux-progression-test-{}", std::process::id()));
    let engine = Engine::boot(EngineConfig { pob_root: root, user_dir, cache_dir: None, data_dir: None }).unwrap();
    let call = |method: &str, params: Value| engine.call(method, &params).unwrap_or_else(|error| panic!("{method}: {error}"));
    call("new_build", json!({"name": "Progression regression fixture"}));
    call("set_level", json!({"level": 80}));
    let group = call("add_socket_group", json!({"label": "Main", "slot": "Body Armour"}));
    call("add_gem", json!({"groupIndex": group["groupIndex"], "nameSpec": "Fireball", "level": 1}));
    call("set_main_skill", json!({"index": group["groupIndex"]}));
    call("set_config", json!({"var": "useFrenzyCharges", "value": true}));
    call("set_config", json!({"var": "conditionKilledRecently", "value": true}));
    call("equip_item_raw", json!({"slot": "Helmet", "text": "Rarity: NORMAL\nIron Hat\n--------\nQuality: +0%\nArmour: 9\n--------\nItem Level: 1\n"}));
    call("equip_item_raw", json!({"slot": "Flask 1", "text": "Rarity: NORMAL\nRuby Flask\n--------\nItem Level: 80\n"}));
    call("set_slot_active", json!({"slot": "Flask 1", "active": true}));
    let before = call("save_build_xml", Value::Null);
    call("compare_add", json!({"xml": before["xml"], "label": "Existing comparison"}));
    let comparisons = call("compare_list", Value::Null);
    let stats_before = call("get_stats", Value::Null);
    let snapshot = call("progression_capture", json!({"label": "Current", "boss": "Pinnacle"}));
    assert_eq!(snapshot["game"], "poe1");
    assert_eq!(snapshot["mainSkill"], "Fireball");
    assert!(snapshot["stats"]["CombinedDPS"].as_f64().unwrap() > 0.0);
    for key in ["CombinedDPS", "Life", "Str", "Dex", "Int"] {
        assert_eq!(snapshot["stats"][key], stats_before["stats"][key], "configured {key} must match live PoB");
    }
    assert_eq!(snapshot["boss"]["overrides"]["useFrenzyCharges"], false);
    assert_eq!(snapshot["boss"]["overrides"]["conditionKilledRecently"], false);
    assert_eq!(snapshot["config"]["useFrenzyCharges"], true);
    assert!(snapshot["equipment"].as_array().unwrap().iter().any(|i| i["slot"] == "Helmet"));
    assert_eq!(call("save_build_xml", Value::Null), before);
    assert_eq!(call("compare_list", Value::Null), comparisons);
    assert_eq!(call("get_stats", Value::Null)["stats"], stats_before["stats"]);

    for invalid in ["not XML", "<PathOfBuilding2><Build/></PathOfBuilding2>", "<PathOfBuilding/>"] {
        assert!(engine.call("progression_capture", &json!({"xml": invalid})).is_err());
        assert_eq!(call("save_build_xml", Value::Null), before);
        assert_eq!(call("compare_list", Value::Null), comparisons);
    }

    let imported = call("progression_capture", json!({"xml": before["xml"], "boss": "Pinnacle"}));
    assert_eq!(imported["stats"], snapshot["stats"]);
    assert_eq!(imported["boss"], snapshot["boss"]);

    // The scenario must equal the original engine with the same explicit inputs.
    for (key, value) in snapshot["boss"]["overrides"].as_object().unwrap() {
        call("set_config", json!({"var": key, "value": value}));
    }
    call("set_slot_active", json!({"slot": "Flask 1", "active": false}));
    let expected = call("get_stats", Value::Null);
    for key in ["CombinedDPS", "Life", "FireResist", "Speed"] {
        assert_eq!(snapshot["boss"]["stats"][key], expected["stats"][key], "boss {key} must match direct PoB calculation");
    }
}
