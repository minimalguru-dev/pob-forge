//! Prices for uniques, currency and divination cards from poe.ninja (PoE1). Whole category lists
//! are downloaded and kept for half an hour, so poe.ninja never learns which item was copied.
//! Rares have no list price; they are left to the trade search.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::game::Game;

const HOST: &str = "https://poe.ninja/poe1/api";
const UA: &str = concat!("PoB Forge/", env!("CARGO_PKG_VERSION"));
const KEEP: Duration = Duration::from_secs(30 * 60);

/// The category lists one rarity is looked up in, in order: (endpoint, type).
fn sources(rarity: &str) -> &'static [(&'static str, &'static str)] {
    match rarity {
        "Unique" => &[
            ("item", "UniqueAccessory"),
            ("item", "UniqueArmour"),
            ("item", "UniqueWeapon"),
            ("item", "UniqueJewel"),
            ("item", "UniqueFlask"),
            ("item", "UniqueMap"),
        ],
        "Currency" => &[
            ("currency", "Currency"),
            ("currency", "Fragment"),
            ("item", "Essence"),
            ("item", "Fossil"),
            ("item", "Resonator"),
            ("item", "Scarab"),
            ("item", "Oil"),
            ("item", "DeliriumOrb"),
            ("item", "Omen"),
            ("item", "Tattoo"),
        ],
        "Divination Card" => &[("item", "DivinationCard")],
        // Normal fragments and scarabs copy as Normal.
        "Normal" => &[("currency", "Fragment"), ("item", "Scarab")],
        _ => &[],
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct Line {
    #[serde(alias = "currencyTypeName")]
    name: String,
    base_type: Option<String>,
    #[serde(alias = "chaosEquivalent")]
    chaos_value: Option<f64>,
    divine_value: Option<f64>,
    links: Option<u32>,
    listing_count: Option<u32>,
    count: Option<u32>,
}

#[derive(Deserialize)]
struct Overview {
    lines: Vec<Line>,
}

#[derive(Deserialize)]
struct League {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexState {
    economy_leagues: Vec<League>,
}

/// One copied item's price.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Price {
    pub name: String,
    pub league: String,
    /// Chaos per unit; the cheapest variant when the unique has several.
    pub chaos: f64,
    /// The dearest variant, when variants differ.
    pub chaos_max: Option<f64>,
    pub divine: Option<f64>,
    pub stack: Option<u32>,
    pub listings: Option<u32>,
}

/// What the overlay needs from item text: rarity, name, base and stack size.
#[derive(Debug, PartialEq)]
pub struct Parsed {
    pub rarity: String,
    pub name: String,
    pub base: Option<String>,
    pub stack: Option<u32>,
}

pub fn parse(raw: &str) -> Option<Parsed> {
    let lines: Vec<&str> = raw.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let at = lines.iter().position(|l| l.starts_with("Rarity:"))?;
    let rarity = lines[at].trim_start_matches("Rarity:").trim().to_string();
    let name = lines.get(at + 1).filter(|l| **l != "--------")?.to_string();
    let base = lines.get(at + 2).filter(|l| **l != "--------").map(|s| s.to_string());
    let stack = lines
        .iter()
        .find_map(|l| l.strip_prefix("Stack Size:"))
        .and_then(|s| s.trim().split('/').next())
        .and_then(|n| n.replace([',', '.', '\u{a0}'], "").trim().parse().ok());
    Some(Parsed { rarity, name, base, stack })
}

type Cache = HashMap<(String, &'static str), (Instant, Vec<Line>)>;
static CACHE: Mutex<Option<Cache>> = Mutex::new(None);
static LEAGUE: Mutex<Option<(Instant, String)>> = Mutex::new(None);

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().user_agent(UA).timeout(Duration::from_secs(20)).build().map_err(|e| e.to_string())
}

async fn get_json<T: for<'de> Deserialize<'de>>(client: &reqwest::Client, url: &str) -> Result<T, String> {
    let resp = client.get(url).send().await.map_err(|e| format!("poe.ninja: {e}"))?;
    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(format!("poe.ninja returned HTTP {status}"));
    }
    resp.json::<T>().await.map_err(|e| format!("poe.ninja: {e}"))
}

/// The current challenge league, as poe.ninja lists it first.
async fn current_league(client: &reqwest::Client) -> Result<String, String> {
    if let Some((at, name)) = LEAGUE.lock().unwrap_or_else(|e| e.into_inner()).clone() {
        if at.elapsed() < KEEP {
            return Ok(name);
        }
    }
    let state: IndexState = get_json(client, &format!("{HOST}/data/index-state")).await?;
    let name = state.economy_leagues.into_iter().next().map(|l| l.name).ok_or("poe.ninja lists no league")?;
    *LEAGUE.lock().unwrap_or_else(|e| e.into_inner()) = Some((Instant::now(), name.clone()));
    Ok(name)
}

async fn lines(client: &reqwest::Client, league: &str, endpoint: &str, kind: &'static str) -> Result<Vec<Line>, String> {
    let key = (format!("{league}/{endpoint}"), kind);
    if let Some((at, lines)) = CACHE.lock().unwrap_or_else(|e| e.into_inner()).as_ref().and_then(|c| c.get(&key)).cloned() {
        if at.elapsed() < KEEP {
            return Ok(lines);
        }
    }
    let url = reqwest::Url::parse_with_params(&format!("{HOST}/economy/stash/current/{endpoint}/overview"), [("league", league), ("type", kind)])
        .map_err(|e| e.to_string())?;
    let overview: Overview = get_json(client, url.as_str()).await?;
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    cache.get_or_insert_with(HashMap::new).insert(key, (Instant::now(), overview.lines.clone()));
    Ok(overview.lines)
}

/// The lines that are this item: by name, then by base for uniques, leaving out the linked-socket variants.
fn matching<'a>(lines: &'a [Line], item: &Parsed) -> Vec<&'a Line> {
    lines
        .iter()
        .filter(|l| l.name == item.name && l.links.unwrap_or(0) < 5)
        .filter(|l| item.rarity != "Unique" || item.base.is_none() || l.base_type.is_none() || l.base_type == item.base)
        .filter(|l| l.chaos_value.is_some_and(|c| c > 0.0))
        .collect()
}

/// The poe.ninja price of copied item text, or `None` when poe.ninja does not list it (rares, gems, PoE2).
pub async fn price(game: Game, raw: &str, league: Option<&str>) -> Result<Option<Price>, String> {
    if game != Game::Poe1 {
        return Ok(None);
    }
    let Some(item) = parse(raw) else { return Ok(None) };
    let wanted = sources(&item.rarity);
    if wanted.is_empty() {
        return Ok(None);
    }
    let client = client()?;
    let league = match league.map(str::trim).filter(|l| !l.is_empty()) {
        Some(l) => l.to_string(),
        None => current_league(&client).await?,
    };
    let mut divine_rate = None;
    for (endpoint, kind) in wanted {
        let list = lines(&client, &league, endpoint, kind).await?;
        if *kind == "Currency" {
            divine_rate = list.iter().find(|l| l.name == "Divine Orb").and_then(|l| l.chaos_value);
        }
        let found = matching(&list, &item);
        if found.is_empty() {
            continue;
        }
        let chaos: Vec<f64> = found.iter().filter_map(|l| l.chaos_value).collect();
        let min = chaos.iter().copied().fold(f64::INFINITY, f64::min);
        let max = chaos.iter().copied().fold(0.0, f64::max);
        let cheapest = found.iter().find(|l| l.chaos_value == Some(min)).copied();
        let divine = cheapest.and_then(|l| l.divine_value).or_else(|| divine_rate.map(|r| min / r)).filter(|d| *d >= 0.05);
        return Ok(Some(Price {
            name: item.name.clone(),
            league,
            chaos: min,
            chaos_max: (max > min * 1.05).then_some(max),
            divine,
            stack: item.stack.filter(|s| *s > 1),
            listings: cheapest.and_then(|l| l.listing_count.or(l.count)),
        }));
    }
    Ok(None)
}

#[tauri::command]
pub async fn price_check(app: tauri::AppHandle, raw: String) -> Result<Option<Price>, String> {
    use tauri::Manager;
    let game = app.state::<crate::AppState>().game();
    let league = crate::game::load_settings(&app).overlay.price_league;
    price(game, &raw, league.as_deref()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_names_bases_and_stacks() {
        let ring = "Item Class: Rings\nRarity: Unique\nKaom's Way\nCoral Ring\n--------\nItem Level: 80";
        assert_eq!(
            parse(ring),
            Some(Parsed { rarity: "Unique".into(), name: "Kaom's Way".into(), base: Some("Coral Ring".into()), stack: None })
        );
        let orbs = "Item Class: Stackable Currency\nRarity: Currency\nDivine Orb\n--------\nStack Size: 3/20\n--------\nRandomises the numeric values";
        let p = parse(orbs).unwrap();
        assert_eq!((p.name.as_str(), p.base, p.stack), ("Divine Orb", None, Some(3)));
        assert_eq!(parse("Rarity: Rare\n--------"), None);
    }

    /// Talks to poe.ninja: `cargo test -p pob-forge --lib prices -- --ignored`.
    #[test]
    #[ignore]
    fn live_prices_for_a_unique_and_a_stack() {
        pob_engine::install_tls_provider();
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let ring = rt
            .block_on(price(Game::Poe1, "Item Class: Rings\nRarity: Unique\nKaom's Way\nCoral Ring\n--------\nItem Level: 80", Some("Standard")))
            .unwrap()
            .expect("Kaom's Way is listed");
        assert!(ring.chaos > 0.0, "{ring:?}");
        let orbs = rt
            .block_on(price(Game::Poe1, "Item Class: Stackable Currency\nRarity: Currency\nDivine Orb\n--------\nStack Size: 3/20", None))
            .unwrap()
            .expect("Divine Orb is listed");
        assert_eq!(orbs.stack, Some(3));
        assert!(orbs.chaos > 1.0, "{orbs:?}");
        let rare = rt.block_on(price(Game::Poe1, "Item Class: Rings\nRarity: Rare\nGale Loop\nIron Ring\n--------", None)).unwrap();
        assert_eq!(rare, None);
        println!("{ring:?}\n{orbs:?}");
    }

    #[test]
    fn unique_variants_match_by_base_and_skip_links() {
        let line = |name: &str, base: &str, chaos: f64, links: Option<u32>| Line {
            name: name.into(),
            base_type: Some(base.into()),
            chaos_value: Some(chaos),
            divine_value: None,
            links,
            listing_count: Some(3),
            count: None,
        };
        let lines = vec![line("Tabula Rasa", "Simple Robe", 10.0, None), line("Tabula Rasa", "Simple Robe", 90.0, Some(6)), line("Other", "Simple Robe", 1.0, None)];
        let item = Parsed { rarity: "Unique".into(), name: "Tabula Rasa".into(), base: Some("Simple Robe".into()), stack: None };
        let found = matching(&lines, &item);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].chaos_value, Some(10.0));
    }
}
