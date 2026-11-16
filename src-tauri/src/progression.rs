//! PoE1 Progression history: snapshots of whole builds, the current/target pair, and the rules that
//! compare them. Kept here so the Progression tab, the overlay and the assistant's tools share one copy.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use tauri::{AppHandle, Emitter, Manager};

pub const MAX_SNAPSHOTS: usize = 20;
pub const MAX_BYTES: usize = 4 * 1024 * 1024;
pub const CHANGED: &str = "progression:changed";

/// Serialises every read-modify-write of the history file.
static LOCK: Mutex<()> = Mutex::new(());

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: i64,
    pub name: String,
    pub mastery: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Equipment {
    pub slot: String,
    pub name: String,
    pub raw: String,
    #[serde(default)]
    pub variant: Option<i64>,
    #[serde(default)]
    pub variant2: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Gem {
    pub group: Number,
    pub name: String,
    pub level: Number,
    pub quality: Number,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Loadout {
    pub tree: String,
    pub items: String,
    pub skills: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BossProfile {
    pub kind: String,
    pub stats: Map<String, Value>,
    pub overrides: Map<String, Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub id: String,
    pub at: String,
    pub xml: String,
    pub label: String,
    pub game: String,
    pub pob_version: String,
    pub tree_version: String,
    pub level: Number,
    pub class_name: String,
    pub ascendancy: String,
    pub main_skill: Option<String>,
    pub nodes: Vec<Node>,
    pub equipment: Vec<Equipment>,
    pub gems: Vec<Gem>,
    pub gem_errors: Vec<String>,
    pub config: Map<String, Value>,
    pub stats: Map<String, Value>,
    pub loadout: Loadout,
    pub boss: BossProfile,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub schema: u32,
    pub snapshots: Vec<Snapshot>,
    pub current_id: Option<String>,
    pub target_id: Option<String>,
}

impl Default for Plan {
    fn default() -> Self {
        Plan { schema: 1, snapshots: Vec::new(), current_id: None, target_id: None }
    }
}

impl Plan {
    pub fn current(&self) -> Option<&Snapshot> {
        self.current_id.as_deref().and_then(|id| self.snapshots.iter().find(|s| s.id == id))
    }
    pub fn target(&self) -> Option<&Snapshot> {
        self.target_id.as_deref().and_then(|id| self.snapshots.iter().find(|s| s.id == id))
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Blocked,
    Review,
    Pass,
}

#[derive(Serialize, Clone, Debug)]
pub struct Check {
    pub id: String,
    pub status: Status,
    pub title: String,
    pub detail: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct SlotChange {
    pub slot: String,
    pub current: Option<Equipment>,
    pub target: Option<Equipment>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Diff {
    pub equipment: Vec<SlotChange>,
    pub compatible_tree: bool,
    pub gained: Vec<Node>,
    pub lost: Vec<Node>,
    pub masteries: Vec<Node>,
    pub config: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Report {
    pub diff: Diff,
    pub checks: Vec<Check>,
    pub priorities: Vec<Check>,
}

fn is_scalar(v: &Value) -> bool {
    matches!(v, Value::String(_) | Value::Bool(_) | Value::Number(_))
}

fn is_stat(v: &Value) -> bool {
    matches!(v, Value::Null | Value::Bool(_) | Value::Number(_))
}

fn valid_time(at: &str) -> bool {
    let re = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(:\d{2}(\.\d+)?)?(Z|[+-]\d{2}:\d{2})$").unwrap();
    re.is_match(at)
}

pub fn validate(plan: &Plan) -> Result<(), String> {
    if plan.schema != 1 || plan.snapshots.len() > MAX_SNAPSHOTS {
        return Err("Unsupported Progression history format.".into());
    }
    let invalid = || "Invalid Progression snapshot. Original history was not changed.".to_string();
    let mut ids = HashSet::new();
    for s in &plan.snapshots {
        let nodes: HashSet<i64> = s.nodes.iter().map(|n| n.id).collect();
        let slots: HashSet<&str> = s.equipment.iter().map(|i| i.slot.as_str()).collect();
        let ok = s.game == "poe1"
            && !s.id.is_empty()
            && ids.insert(s.id.as_str())
            && valid_time(&s.at)
            && ["Boss", "Pinnacle", "Uber"].contains(&s.boss.kind.as_str())
            && s.stats.values().all(is_stat)
            && s.boss.stats.values().all(is_stat)
            && s.config.values().all(is_scalar)
            && s.boss.overrides.values().all(is_scalar)
            && nodes.len() == s.nodes.len()
            && slots.len() == s.equipment.len();
        if !ok {
            return Err(invalid());
        }
    }
    let missing = |id: &Option<String>| id.as_deref().is_some_and(|id| !ids.contains(id));
    if missing(&plan.current_id) || missing(&plan.target_id) {
        return Err("History refers to a missing snapshot.".into());
    }
    Ok(())
}

pub fn decode(text: &str) -> Result<Plan, String> {
    if text.len() > MAX_BYTES {
        return Err("Progression data exceeds 4 MiB.".into());
    }
    let plan: Plan = serde_json::from_str(text).map_err(|_| "Unsupported Progression history format.".to_string())?;
    validate(&plan)?;
    Ok(plan)
}

fn encode(plan: &Plan) -> Result<String, String> {
    validate(plan)?;
    let text = serde_json::to_string(plan).map_err(|e| e.to_string())?;
    if text.len() > MAX_BYTES {
        return Err("Progression history would exceed 4 MiB. Export a backup and remove a snapshot first.".into());
    }
    Ok(text)
}

pub fn number(stats: &Map<String, Value>, key: &str) -> Option<f64> {
    stats.get(key).and_then(Value::as_f64).filter(|v| v.is_finite())
}

fn truthy(stats: &Map<String, Value>, key: &str) -> bool {
    match stats.get(key) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|v| v != 0.0),
        _ => false,
    }
}

fn shown(v: Option<f64>) -> String {
    v.map(|v| format!("{v}")).unwrap_or_else(|| "Unknown".into())
}

fn signature(item: Option<&Equipment>) -> Option<(String, Option<i64>, Option<i64>)> {
    item.map(|i| (i.raw.replace("\r\n", "\n").trim().to_string(), i.variant, i.variant2))
}

pub fn diff(current: &Snapshot, target: &Snapshot) -> Diff {
    let a: HashMap<&str, &Equipment> = current.equipment.iter().map(|i| (i.slot.as_str(), i)).collect();
    let b: HashMap<&str, &Equipment> = target.equipment.iter().map(|i| (i.slot.as_str(), i)).collect();
    let slots: BTreeSet<&str> = a.keys().chain(b.keys()).copied().collect();
    let equipment = slots
        .into_iter()
        .filter(|slot| signature(a.get(slot).copied()) != signature(b.get(slot).copied()))
        .map(|slot| SlotChange { slot: slot.to_string(), current: a.get(slot).map(|&i| i.clone()), target: b.get(slot).map(|&i| i.clone()) })
        .collect();
    let compatible_tree = current.tree_version == target.tree_version && current.class_name == target.class_name;
    let mine: HashMap<i64, &Node> = current.nodes.iter().map(|n| (n.id, n)).collect();
    let theirs: HashSet<i64> = target.nodes.iter().map(|n| n.id).collect();
    let keys: BTreeSet<&String> = current.config.keys().chain(target.config.keys()).collect();
    Diff {
        equipment,
        compatible_tree,
        gained: if compatible_tree { target.nodes.iter().filter(|n| !mine.contains_key(&n.id)).cloned().collect() } else { vec![] },
        lost: if compatible_tree { current.nodes.iter().filter(|n| !theirs.contains(&n.id)).cloned().collect() } else { vec![] },
        masteries: if compatible_tree {
            target.nodes.iter().filter(|n| mine.get(&n.id).is_some_and(|m| m.mastery != n.mastery)).cloned().collect()
        } else {
            vec![]
        },
        config: keys.into_iter().filter(|k| current.config.get(*k) != target.config.get(*k)).cloned().collect(),
    }
}

pub fn assess(current: &Snapshot, target: &Snapshot) -> Vec<Check> {
    let d = diff(current, target);
    let mut checks = Vec::new();
    let mut add = |id: String, status: Status, title: String, detail: String| checks.push(Check { id, status, title, detail });
    let pass_if = |ok: bool, otherwise: Status| if ok { Status::Pass } else { otherwise };
    add("class".into(), pass_if(current.class_name == target.class_name, Status::Blocked), "Character class".into(),
        format!("{} → {}. A different base class requires another character.", current.class_name, target.class_name));
    add("ascendancy".into(), pass_if(current.ascendancy == target.ascendancy, Status::Review), "Ascendancy".into(),
        format!("{} → {}", current.ascendancy, target.ascendancy));
    add("versions".into(), pass_if(current.pob_version == target.pob_version && d.compatible_tree, Status::Review), "Engine and tree versions".into(),
        "Re-capture both builds with the same PoB version before comparing. Tree node IDs from different trees are not compared.".into());
    let (cl, tl) = (current.level.as_f64().unwrap_or(0.0), target.level.as_f64().unwrap_or(0.0));
    add("level".into(), pass_if(cl >= tl, Status::Review), "Target level".into(),
        format!("Current {}; target {}. Select an appropriate earlier target loadout if needed.", current.level, target.level));
    add("gear".into(), pass_if(d.equipment.is_empty(), Status::Review), "Target equipment".into(),
        format!("{} slot changes. Different gear is not necessarily worse; ownership and budget are not known.", d.equipment.len()));
    let mut missing: Vec<&str> = Vec::new();
    for gem in target.gems.iter().filter(|g| g.enabled) {
        if !current.gems.iter().any(|g| g.enabled && g.name == gem.name) && !missing.contains(&gem.name.as_str()) {
            missing.push(&gem.name);
        }
    }
    add("gems".into(), pass_if(missing.is_empty(), Status::Review), "Target gems".into(), if missing.is_empty() {
        "Enabled target gem names are present. Verify their levels, supports and socket links.".into()
    } else {
        format!("Prepare: {}. Verify links, colours and gem levels in Skills.", missing.join(", "))
    });
    let tree_same = d.compatible_tree && d.gained.is_empty() && d.lost.is_empty() && d.masteries.is_empty();
    add("tree".into(), pass_if(tree_same, Status::Review), "Passive transition".into(), if d.compatible_tree {
        format!("Add {}, remove {}, change {} masteries. Node counts are not a refund-cost estimate.", d.gained.len(), d.lost.len(), d.masteries.len())
    } else {
        "Different tree versions or classes: node diff unavailable.".into()
    });
    for (side, s) in [("Current", current), ("Target", target)] {
        for attr in ["Str", "Dex", "Int"] {
            let (have, need) = (number(&s.stats, attr), number(&s.stats, &format!("Req{attr}")));
            let status = match (have, need) {
                (Some(h), Some(n)) if h < n => Status::Blocked,
                (Some(_), Some(_)) => Status::Pass,
                _ => Status::Review,
            };
            add(format!("{side}-{attr}"), status, format!("{side} {attr} requirements"),
                format!("{} available / {} required by PoB.", shown(have), shown(need)));
        }
        if truthy(&s.stats, "ManaCostWarning") || truthy(&s.stats, "LifeCostWarning") {
            add(format!("{side}-cost"), Status::Blocked, format!("{side} skill resource cost"),
                "PoB reports that the skill costs more than the unreserved pool. Resolve this in Config / Skills before switching.".into());
        }
        if !s.gem_errors.is_empty() {
            add(format!("{side}-errors"), Status::Blocked, format!("{side} gem errors"), s.gem_errors.join("; "));
        }
        let dps = number(&s.stats, "CombinedDPS").filter(|v| *v != 0.0).or_else(|| number(&s.stats, "TotalDPS").filter(|v| *v != 0.0));
        if s.main_skill.is_none() || dps.is_none() {
            add(format!("{side}-skill"), Status::Review, format!("{side} main skill"),
                "No positive main-skill DPS reported. Select the intended damage skill and inspect Calcs.".into());
        }
        for resist in ["Fire", "Cold", "Lightning"] {
            let (value, cap) = (number(&s.stats, &format!("{resist}Resist")), number(&s.stats, &format!("{resist}ResistMax")));
            if !matches!((value, cap), (Some(v), Some(c)) if v >= c) {
                add(format!("{side}-{resist}"), Status::Review, format!("{side} {} resistance", resist.to_lowercase()),
                    format!("{}% / {}% PoB cap. Check the boss profile without flasks too.",
                        value.map(|v| format!("{v}")).unwrap_or_else(|| "Unknown".into()),
                        cap.map(|v| format!("{v}")).unwrap_or_else(|| "unknown".into())));
            }
        }
    }
    if !d.config.is_empty() {
        add("config".into(), Status::Review, "Different configuration".into(),
            format!("{} options differ. Configured DPS is not a like-for-like upgrade estimate.", d.config.len()));
    }
    add("manual".into(), Status::Review, "Validate the transition in play".into(),
        "Confirm item ownership, links, refund budget, recovery, ailment protection, accuracy and sustainable charges. A target build is not a simulation of equipping its items on your current character.".into());
    checks
}

fn rank(check: &Check) -> u8 {
    let id = check.id.as_str();
    if check.status == Status::Blocked {
        0
    } else if id.contains("versions") || id.contains("config") {
        1
    } else if id.contains("gear") || id.contains("gems") || id.contains("tree") || id.starts_with("item-Weapon") {
        2
    } else {
        3
    }
}

pub fn priorities(current: &Snapshot, target: &Snapshot) -> Vec<Check> {
    let mut checks: Vec<Check> = assess(current, target).into_iter().filter(|c| c.status != Status::Pass).collect();
    for change in diff(current, target).equipment {
        let Some(t) = &change.target else { continue };
        checks.push(Check {
            id: format!("item-{}", change.slot),
            status: Status::Review,
            title: format!("Prepare {}: {}", change.slot, t.name),
            detail: format!(
                "Compare against {} in PoB before buying or equipping. Check attribute requirements, resistances and skill compatibility together; no isolated DPS gain has been calculated.",
                change.current.as_ref().map(|c| c.name.as_str()).unwrap_or("the empty slot")
            ),
        });
    }
    checks.sort_by_key(rank);
    checks
}

pub fn report(current: &Snapshot, target: &Snapshot) -> Report {
    Report { diff: diff(current, target), checks: assess(current, target), priorities: priorities(current, target) }
}

pub fn add(plan: &Plan, snapshot: Snapshot, role: &str) -> Result<Plan, String> {
    if plan.snapshots.len() >= MAX_SNAPSHOTS {
        return Err("History is full (20 snapshots). Export a backup and remove a snapshot first.".into());
    }
    let mut next = plan.clone();
    let id = snapshot.id.clone();
    next.snapshots.push(snapshot);
    match role {
        "current" => next.current_id = Some(id),
        "target" => next.target_id = Some(id),
        _ => return Err(format!("unknown role {role}: use current or target")),
    }
    Ok(next)
}

pub fn remove(plan: &Plan, id: &str) -> Plan {
    let clear = |sel: &Option<String>| sel.clone().filter(|s| s != id);
    Plan {
        schema: plan.schema,
        snapshots: plan.snapshots.iter().filter(|s| s.id != id).cloned().collect(),
        current_id: clear(&plan.current_id),
        target_id: clear(&plan.target_id),
    }
}

/// Backup import: adds snapshots it does not already hold and keeps the existing selection.
pub fn merge(plan: &Plan, imported: &Plan) -> Plan {
    let mut next = plan.clone();
    for s in &imported.snapshots {
        if !next.snapshots.iter().any(|e| e.id == s.id) {
            next.snapshots.push(s.clone());
        }
    }
    next.current_id = plan.current_id.clone().or_else(|| imported.current_id.clone());
    next.target_id = plan.target_id.clone().or_else(|| imported.target_id.clone());
    next
}

/// UTC time in the same format JavaScript's `toISOString` writes.
pub fn now_iso() -> String {
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let (secs, millis) = (d.as_secs() as i64, d.subsec_millis());
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{millis:03}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

pub fn new_id() -> String {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).unwrap_or_default();
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

/// Turns `progression_capture`'s result into a stored snapshot.
pub fn from_capture(mut captured: Value) -> Result<Snapshot, String> {
    let obj = captured.as_object_mut().ok_or("capture returned no snapshot")?;
    obj.insert("id".into(), Value::String(new_id()));
    obj.insert("at".into(), Value::String(now_iso()));
    serde_json::from_value(captured).map_err(|e| format!("capture returned an unexpected snapshot: {e}"))
}

fn path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().app_local_data_dir().map_err(|e| e.to_string())?.join("progression-poe1.json"))
}

/// `None` when nothing has been saved yet, so the view can migrate its old local storage.
pub fn load(app: &AppHandle) -> Result<Option<Plan>, String> {
    let path = path(app)?;
    match std::fs::read_to_string(&path) {
        Ok(text) => decode(&text).map(Some),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("could not read {}: {e}", path.display())),
    }
}

fn store(app: &AppHandle, plan: &Plan) -> Result<(), String> {
    let text = encode(plan)?;
    let path = path(app)?;
    crate::art::write_atomic(&path, text.as_bytes()).map_err(|e| format!("could not write {}: {e}", path.display()))?;
    let _ = app.emit(CHANGED, ());
    Ok(())
}

/// Loads, applies `change` and saves under one lock. An unreadable history is never overwritten.
pub fn update(app: &AppHandle, change: impl FnOnce(&Plan) -> Result<Plan, String>) -> Result<Plan, String> {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let plan = load(app)?.unwrap_or_default();
    let next = change(&plan)?;
    store(app, &next)?;
    Ok(next)
}

#[tauri::command]
pub fn progression_load(app: AppHandle) -> Result<Option<Plan>, String> {
    load(&app)
}

/// One-off move of the history the view kept in local storage. Refuses when a file already exists.
#[tauri::command]
pub fn progression_migrate(app: AppHandle, text: String) -> Result<Plan, String> {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(existing) = load(&app)? {
        return Ok(existing);
    }
    let plan = decode(&text)?;
    store(&app, &plan)?;
    Ok(plan)
}

#[tauri::command]
pub fn progression_decode(text: String) -> Result<Plan, String> {
    decode(&text)
}

#[tauri::command]
pub fn progression_add(app: AppHandle, captured: Value, role: String) -> Result<Plan, String> {
    let snapshot = from_capture(captured)?;
    update(&app, |plan| add(plan, snapshot, &role))
}

#[tauri::command]
pub fn progression_remove(app: AppHandle, id: String) -> Result<Plan, String> {
    update(&app, |plan| Ok(remove(plan, &id)))
}

#[tauri::command]
pub fn progression_select(app: AppHandle, current_id: Option<String>, target_id: Option<String>) -> Result<Plan, String> {
    update(&app, |plan| {
        let mut next = plan.clone();
        if current_id.is_some() {
            next.current_id = current_id;
        }
        if target_id.is_some() {
            next.target_id = target_id;
        }
        Ok(next)
    })
}

#[tauri::command]
pub fn progression_import(app: AppHandle, text: String) -> Result<Plan, String> {
    let imported = decode(&text)?;
    update(&app, |plan| Ok(merge(plan, &imported)))
}

#[tauri::command]
pub fn progression_report(current: Snapshot, target: Snapshot) -> Report {
    report(&current, &target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn snapshot(patch: Value) -> Snapshot {
        let mut base = json!({
            "id": "one", "at": "2026-11-15T12:00:00Z", "xml": "<PathOfBuilding><Build/></PathOfBuilding>", "label": "Test fixture", "game": "poe1",
            "pobVersion": "test", "treeVersion": "test-tree", "level": 80, "className": "Duelist", "ascendancy": "Slayer", "mainSkill": "Shield Crush",
            "nodes": [{ "id": 1, "name": "Start", "mastery": null }], "equipment": [], "gems": [], "gemErrors": [], "config": {},
            "stats": { "Str": 100, "Dex": 100, "Int": 100, "ReqStr": 50, "ReqDex": 50, "ReqInt": 50, "CombinedDPS": 10,
                "FireResist": 75, "ColdResist": 75, "LightningResist": 75, "FireResistMax": 75, "ColdResistMax": 75, "LightningResistMax": 75 },
            "loadout": { "tree": "Default", "items": "1", "skills": "1" }, "boss": { "kind": "Pinnacle", "stats": {}, "overrides": {} },
        });
        for (k, v) in patch.as_object().unwrap() {
            base[k] = v.clone();
        }
        serde_json::from_value(base).unwrap()
    }

    fn item(raw: &str, slot: &str) -> Value {
        json!({ "slot": slot, "name": "Same rare name", "raw": raw, "variant": null, "variant2": null })
    }

    fn check<'a>(checks: &'a [Check], id: &str) -> Option<&'a Check> {
        checks.iter().find(|c| c.id == id)
    }

    #[test]
    fn equipment_diff_detects_changed_rolls_and_slots() {
        let a = snapshot(json!({ "equipment": [item("10% attack speed", "Weapon 1"), item("shield", "Weapon 2")] }));
        let b = snapshot(json!({ "equipment": [item("20% attack speed", "Weapon 1"), item("ring", "Ring 1")] }));
        let slots: Vec<_> = diff(&a, &b).equipment.into_iter().map(|c| c.slot).collect();
        assert_eq!(slots, ["Ring 1", "Weapon 1", "Weapon 2"]);
    }

    #[test]
    fn line_endings_normalise_but_variants_count() {
        let a = snapshot(json!({ "equipment": [item("a\r\nb", "Weapon 1")] }));
        let b = snapshot(json!({ "equipment": [item("a\nb", "Weapon 1")] }));
        assert!(diff(&a, &b).equipment.is_empty());
        let mut v = item("a", "Weapon 1");
        v["variant"] = json!(2);
        assert_eq!(diff(&snapshot(json!({ "equipment": [item("a", "Weapon 1")] })), &snapshot(json!({ "equipment": [v] }))).equipment.len(), 1);
    }

    #[test]
    fn tree_diff_separates_gained_lost_and_masteries() {
        let a = snapshot(json!({ "nodes": [{ "id": 1, "name": "Mastery", "mastery": 4 }, { "id": 2, "name": "Old", "mastery": null }] }));
        let b = snapshot(json!({ "nodes": [{ "id": 1, "name": "Mastery", "mastery": 5 }, { "id": 3, "name": "New", "mastery": null }] }));
        let d = diff(&a, &b);
        assert_eq!(d.gained.iter().map(|n| n.id).collect::<Vec<_>>(), [3]);
        assert_eq!(d.lost.iter().map(|n| n.id).collect::<Vec<_>>(), [2]);
        assert_eq!(d.masteries.iter().map(|n| n.id).collect::<Vec<_>>(), [1]);
    }

    #[test]
    fn different_trees_give_no_node_diff() {
        let d = diff(&snapshot(json!({})), &snapshot(json!({ "treeVersion": "other", "nodes": [] })));
        assert!(!d.compatible_tree);
        assert!(d.lost.is_empty());
    }

    #[test]
    fn class_mismatch_and_requirements_block() {
        let checks = assess(&snapshot(json!({})), &snapshot(json!({ "className": "Witch", "stats": { "Int": 20, "ReqInt": 149 } })));
        assert_eq!(check(&checks, "class").unwrap().status, Status::Blocked);
        assert_eq!(check(&checks, "Target-Int").unwrap().status, Status::Blocked);
        assert_eq!(check(&assess(&snapshot(json!({})), &snapshot(json!({}))), "manual").unwrap().status, Status::Review);
    }

    #[test]
    fn missing_attributes_are_unknown() {
        let checks = assess(&snapshot(json!({ "stats": {} })), &snapshot(json!({})));
        assert_eq!(check(&checks, "Current-Str").unwrap().status, Status::Review);
        assert_eq!(number(&Map::new(), "CombinedDPS"), None);
    }

    #[test]
    fn resource_and_gem_errors_outrank_gear() {
        let a = snapshot(json!({ "stats": { "ManaCostWarning": true }, "gemErrors": ["Not enough intelligence"] }));
        let b = snapshot(json!({ "equipment": [item("target", "Weapon 1")] }));
        let list = priorities(&a, &b);
        assert_eq!(list[0].status, Status::Blocked);
        let pos = |id: &str| list.iter().position(|c| c.id == id).unwrap();
        assert!(pos("Current-errors") < pos("gear"));
        assert!(list.iter().any(|c| c.title == "Prepare Weapon 1: Same rare name"));
    }

    #[test]
    fn config_differences_stay_visible() {
        let a = snapshot(json!({ "config": { "useFrenzyCharges": true } }));
        assert_eq!(diff(&a, &snapshot(json!({}))).config, ["useFrenzyCharges"]);
        assert_eq!(check(&assess(&a, &snapshot(json!({}))), "config").unwrap().status, Status::Review);
    }

    #[test]
    fn disabled_gems_do_not_count() {
        let gem = json!({ "group": 1, "name": "Dual Strike of Ambidexterity", "level": 1, "quality": 0, "enabled": false });
        let mut on = gem.clone();
        on["enabled"] = json!(true);
        let checks = assess(&snapshot(json!({ "gems": [gem] })), &snapshot(json!({ "gems": [on] })));
        assert_eq!(check(&checks, "gems").unwrap().status, Status::Review);
    }

    #[test]
    fn history_round_trips_and_removal_clears_selection() {
        let first = add(&Plan::default(), snapshot(json!({})), "current").unwrap();
        let plan = add(&first, snapshot(json!({ "id": "two" })), "target").unwrap();
        assert_eq!(decode(&serde_json::to_string(&plan).unwrap()).unwrap(), plan);
        assert_eq!(remove(&plan, "one").current_id, None);
        assert_eq!(remove(&plan, "one").target_id.as_deref(), Some("two"));
    }

    #[test]
    fn full_history_refuses_instead_of_dropping() {
        let mut plan = Plan::default();
        for i in 0..20 {
            plan = add(&plan, snapshot(json!({ "id": i.to_string() })), "current").unwrap();
        }
        assert!(add(&plan, snapshot(json!({})), "target").unwrap_err().contains("History is full"));
    }

    #[test]
    fn invalid_history_is_rejected() {
        let one = serde_json::to_value(snapshot(json!({}))).unwrap();
        let bad = [
            "broken".to_string(),
            "null".to_string(),
            json!({ "schema": 2, "snapshots": [], "currentId": null, "targetId": null }).to_string(),
            json!({ "schema": 1, "snapshots": [], "currentId": "missing", "targetId": null }).to_string(),
            json!({ "schema": 1, "snapshots": [one, one], "currentId": null, "targetId": null }).to_string(),
            " ".repeat(MAX_BYTES + 1),
        ];
        for text in bad {
            assert!(decode(&text).is_err(), "accepted {}", &text[..text.len().min(60)]);
        }
    }

    #[test]
    fn malformed_snapshots_are_rejected() {
        let base = serde_json::to_value(snapshot(json!({}))).unwrap();
        let patches = [
            json!({ "nodes": [{ "id": 1, "name": "a", "mastery": null }, { "id": 1, "name": "a", "mastery": null }] }),
            json!({ "equipment": [item("a", "Weapon 1"), item("b", "Weapon 1")] }),
            json!({ "stats": { "Life": { "invalid": true } } }),
            json!({ "config": { "enemyIsBoss": ["Pinnacle"] } }),
            json!({ "at": "not a date" }),
            json!({ "loadout": {} }),
        ];
        for patch in patches {
            let mut s = base.clone();
            for (k, v) in patch.as_object().unwrap() {
                s[k] = v.clone();
            }
            let text = json!({ "schema": 1, "snapshots": [s], "currentId": null, "targetId": null }).to_string();
            assert!(decode(&text).is_err(), "accepted {patch}");
        }
    }

    #[test]
    fn merge_keeps_selection_and_skips_known_ids() {
        let mine = add(&Plan::default(), snapshot(json!({})), "current").unwrap();
        let theirs = add(&add(&Plan::default(), snapshot(json!({})), "current").unwrap(), snapshot(json!({ "id": "two" })), "target").unwrap();
        let merged = merge(&mine, &theirs);
        assert_eq!(merged.snapshots.len(), 2);
        assert_eq!(merged.current_id.as_deref(), Some("one"));
        assert_eq!(merged.target_id.as_deref(), Some("two"));
    }

    #[test]
    fn capture_gets_an_id_and_a_valid_time() {
        let mut v = serde_json::to_value(snapshot(json!({}))).unwrap();
        v.as_object_mut().unwrap().remove("id");
        v.as_object_mut().unwrap().remove("at");
        let s = from_capture(v).unwrap();
        assert_eq!(s.id.len(), 36);
        assert!(valid_time(&s.at), "{}", s.at);
        assert_ne!(new_id(), new_id());
    }
}
