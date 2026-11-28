use std::path::PathBuf;

use pob_engine::{Engine, EngineConfig};
use serde_json::{json, Value};

const GLOVES: &str = "Rarity: Rare\nGrip Clutches\nIronscale Gauntlets\n--------\nItem Level: 80\n--------\n+62 to maximum Life\n+35% to Fire Resistance\n";
const OTHER_GLOVES: &str = "Rarity: Rare\nDoom Mitts\nIronscale Gauntlets\n--------\nItem Level: 80\n--------\n+80 to maximum Life\n+20% to Cold Resistance\n";

fn state(engine: &Engine) -> Value {
    json!({
        "xml": engine.call("save_build_xml", &Value::Null).unwrap(),
        "compare": engine.call("compare_list", &Value::Null).unwrap(),
        "undo": engine.eval("local b = launch.main.modes.BUILD; return { items = #(b.itemsTab.undo or {}), skills = #(b.skillsTab.undo or {}), config = #(b.configTab.undo or {}), tree = #(b.spec.undo or {}) }").unwrap(),
    })
}

/// One reference build: Heavy Strike with the given supports, wearing `gloves`.
fn reference(engine: &Engine, name: &str, supports: &[&str], gloves: &str) -> String {
    engine.call("new_build", &json!({ "name": name })).unwrap();
    let group = engine.call("add_socket_group", &json!({})).unwrap();
    for gem in std::iter::once(&"Heavy Strike").chain(supports) {
        engine.call("add_gem", &json!({ "groupIndex": group["groupIndex"], "nameSpec": gem, "level": 1 })).unwrap();
    }
    engine.call("equip_item_raw", &json!({ "text": gloves, "slot": "Gloves" })).unwrap();
    engine.call("save_build_xml", &Value::Null).unwrap()["xml"].as_str().unwrap().to_string()
}

fn row<'a>(list: &'a Value, name: &str) -> Option<&'a Value> {
    list.as_array()?.iter().find(|r| r["name"] == name)
}

#[test]
fn reference_builds_are_counted_against_the_open_build_without_changing_it() {
    let root = std::env::var_os("POB_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/pob1"));
    if !root.join("Launch.lua").is_file() || root.ends_with("pob") {
        eprintln!("skipping: needs the PoE1 PoB (run pob-sync or set POB_ROOT)");
        return;
    }
    let user_dir = std::env::temp_dir().join(format!("pob-reference-builds-{}", std::process::id()));
    let engine = Engine::boot(EngineConfig { pob_root: root, user_dir: user_dir.clone(), cache_dir: None, data_dir: None }).unwrap();

    let a = reference(&engine, "Ref A", &["Melee Physical Damage", "Brutality"], GLOVES);
    let b = reference(&engine, "Ref B", &["Melee Physical Damage", "Ruthless"], OTHER_GLOVES);

    engine.call("new_build", &json!({ "name": "Mine" })).unwrap();
    let group = engine.call("add_socket_group", &json!({})).unwrap();
    engine.call("add_gem", &json!({ "groupIndex": group["groupIndex"], "nameSpec": "Heavy Strike", "level": 1 })).unwrap();
    engine.call("add_gem", &json!({ "groupIndex": group["groupIndex"], "nameSpec": "Brutality", "level": 1 })).unwrap();
    let before = state(&engine);

    let r = engine
        .call(
            "reference_builds",
            &json!({ "builds": [
                { "xml": a, "label": "A" },
                { "xml": b, "label": "B" },
                { "code": "not a build code", "label": "broken" },
            ] }),
        )
        .unwrap();

    assert_eq!(r["count"], 2, "{r}");
    assert_eq!(r["failed"][0]["label"], "broken", "{}", r["failed"]);
    assert_eq!(r["mine"]["mainSkill"], "Heavy Strike");
    assert_eq!(r["builds"][0]["mainSkill"], "Heavy Strike");

    let supports = &r["common"]["supports"];
    let mpd = row(supports, "Melee Physical Damage").unwrap_or_else(|| panic!("{supports}"));
    assert_eq!(mpd["builds"], 2);
    assert_eq!(mpd["mine"], false);
    let brutality = row(supports, "Brutality").unwrap();
    assert_eq!(brutality["builds"], 1);
    assert_eq!(brutality["mine"], true);

    let gloves = r["common"]["gear"].as_array().unwrap().iter().find(|g| g["slot"] == "Gloves").expect("a Gloves row");
    assert_eq!(gloves["rare"], 2);
    assert_eq!(gloves["mine"], Value::Null);
    assert_eq!(row(&gloves["rareBases"], "Ironscale Gauntlets").unwrap()["builds"], 2);
    assert_eq!(row(&gloves["rareMods"], "+# to maximum Life").unwrap_or_else(|| panic!("{}", gloves["rareMods"]))["builds"], 2);
    assert_eq!(row(&gloves["rareMods"], "+#% to Fire Resistance").unwrap()["builds"], 1);

    // Each build's numbers come from the engine: the life on the gloves shows up.
    let life = |v: &Value| v["stats"]["Life"].as_f64().unwrap();
    assert!(life(&r["builds"][1]) > life(&r["builds"][0]), "{} vs {}", r["builds"][1]["stats"], r["builds"][0]["stats"]);
    assert!(life(&r["builds"][0]) > life(&r["mine"]));

    assert_eq!(state(&engine), before, "reading reference builds must leave the build, its undo history and Compare unchanged");

    let err = engine.call("reference_builds", &json!({ "builds": [{ "code": "nope" }] })).unwrap_err().to_string();
    assert!(err.contains("none of those builds could be read"), "{err}");
    let _ = std::fs::remove_dir_all(user_dir);
}
