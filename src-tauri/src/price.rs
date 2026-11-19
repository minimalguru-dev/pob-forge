//! Price checks for the overlay: poe.ninja's price tables for what it tracks, and anonymous
//! searches on pathofexile.com's trade site for the rest. Neither signs in or sends cookies.
//! A trade rule at its limit is waited out instead of sending, as `ggg` does for signed-in calls.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::Value;
use tauri::State;

use crate::game::Game;

const TRADE: &str = "https://www.pathofexile.com/";
const NINJA: &str = "https://poe.ninja/";
/// poe.ninja refreshes its tables about every five minutes.
const NINJA_TTL: Duration = Duration::from_secs(10 * 60);
/// Listings fetched per search: the cheapest ten, as the trade site shows first.
const FETCH: usize = 10;

#[derive(Default)]
pub struct PriceState {
    not_before: Mutex<HashMap<&'static str, Instant>>,
    ninja: Mutex<HashMap<String, (Instant, NinjaTable)>>,
}

impl PriceState {
    fn wait(&self, policy: &str) -> Option<u64> {
        let until = *self.not_before.lock().unwrap().get(policy)?;
        Some(until.checked_duration_since(Instant::now())?.as_secs() + 1)
    }

    fn limit(&self, policy: &'static str, wait: u64) {
        let mut not_before = self.not_before.lock().unwrap();
        if wait == 0 {
            not_before.remove(policy);
        } else {
            not_before.insert(policy, Instant::now() + Duration::from_secs(wait));
        }
    }
}

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(format!("PoB Forge/{} (contact: {})", env!("CARGO_PKG_VERSION"), crate::brand::ggg_contact()))
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(20))
            .build()
            .expect("price http client")
    })
}

/// `https://www.pathofexile.com/<path…>` with each segment encoded.
fn trade_url(segments: &[&str]) -> Result<reqwest::Url, String> {
    let mut url = reqwest::Url::parse(TRADE).map_err(|e| e.to_string())?;
    url.path_segments_mut().map_err(|_| "bad trade url".to_string())?.extend(segments);
    Ok(url)
}

/// API path segments and site path segments for a game's trade search.
fn trade_paths(game: Game) -> (&'static [&'static str], &'static [&'static str]) {
    match game {
        Game::Poe1 => (&["api", "trade"], &["trade"]),
        Game::Poe2 => (&["api", "trade2"], &["trade2"]),
    }
}

fn realm(game: Game) -> Option<&'static str> {
    (game == Game::Poe2).then_some("poe2")
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    amount: Option<f64>,
    currency: Option<String>,
    /// When the listing was indexed, as the trade site gives it (RFC 3339).
    indexed: Option<String>,
    account: Option<String>,
    online: bool,
    stack: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeResult {
    total: u64,
    /// The same search on the trade site, to open in the trade window.
    url: String,
    listings: Vec<Listing>,
}

fn error_message(body: &str) -> Option<String> {
    let v: Value = serde_json::from_str(body).ok()?;
    Some(v.pointer("/error/message")?.as_str()?.to_string())
}

async fn send(state: &PriceState, request: reqwest::RequestBuilder, url: &str) -> Result<String, String> {
    let policy = crate::ggg::trade_policy(url)?;
    if let Some(wait) = state.wait(policy) {
        return Err(crate::ggg::limited(wait));
    }
    let resp = request.send().await.map_err(|e| format!("pathofexile.com: {e}"))?;
    let wait = crate::ggg::wait_from(resp.headers());
    state.limit(policy, wait);
    let status = resp.status().as_u16();
    let body = resp.text().await.map_err(|e| format!("pathofexile.com: {e}"))?;
    match status {
        200..=299 => Ok(body),
        429 => Err(crate::ggg::limited(wait.max(1))),
        _ => Err(error_message(&body).map_or_else(|| format!("pathofexile.com returned HTTP {status}"), |m| format!("pathofexile.com: {m}"))),
    }
}

fn listings(body: &str) -> Result<Vec<Listing>, String> {
    let v: Value = serde_json::from_str(body).map_err(|_| "pathofexile.com sent listings this app could not read".to_string())?;
    let results = v.get("result").and_then(Value::as_array).cloned().unwrap_or_default();
    Ok(results
        .iter()
        .filter(|r| r.is_object())
        .map(|r| {
            let listing = &r["listing"];
            let price = &listing["price"];
            Listing {
                amount: price["amount"].as_f64(),
                currency: price["currency"].as_str().map(str::to_string),
                indexed: listing["indexed"].as_str().map(str::to_string),
                account: listing.pointer("/account/name").and_then(Value::as_str).map(str::to_string),
                online: listing.pointer("/account/online").is_some_and(|o| !o.is_null()),
                stack: r.pointer("/item/stackSize").and_then(Value::as_u64),
            }
        })
        .collect())
}

/// The API wants `{ query, sort }`; PoB's Buy Similar builds the `query` part alone.
fn search_body(query: &str) -> Result<Value, String> {
    let v: Value = serde_json::from_str(query).map_err(|_| "the search is not valid JSON".to_string())?;
    let mut body = match v {
        Value::Object(ref o) if o.get("query").is_some_and(Value::is_object) => v,
        Value::Object(_) => serde_json::json!({ "query": v }),
        _ => return Err("the search has no query".into()),
    };
    if body.get("sort").is_none() {
        body["sort"] = serde_json::json!({ "price": "asc" });
    }
    Ok(body)
}

/// Runs a trade search (PoB's Buy Similar query) and fetches its cheapest listings.
#[tauri::command]
pub async fn price_trade(state: State<'_, PriceState>, game: Game, league: String, query: String) -> Result<TradeResult, String> {
    let body = search_body(&query)?;
    let league = league.trim();
    if league.is_empty() {
        return Err("Choose a league first.".into());
    }
    let (api, site) = trade_paths(game);
    let mut search_path: Vec<&str> = api.to_vec();
    search_path.push("search");
    search_path.extend(realm(game));
    search_path.push(league);
    let search = trade_url(&search_path)?;
    let request = client().post(search.clone()).header("content-type", "application/json").body(body.to_string());
    let found: Value = serde_json::from_str(&send(&state, request, search.as_str()).await?).map_err(|_| "pathofexile.com sent a search reply this app could not read".to_string())?;
    let id = found["id"].as_str().filter(|id| id.bytes().all(|b| b.is_ascii_alphanumeric())).ok_or("pathofexile.com sent no search id")?.to_string();
    let total = found["total"].as_u64().unwrap_or(0);
    let ids: Vec<&str> = found["result"].as_array().map(|a| a.iter().filter_map(Value::as_str).take(FETCH).collect()).unwrap_or_default();

    let mut site_path: Vec<&str> = site.to_vec();
    site_path.push("search");
    site_path.extend(realm(game));
    site_path.push(league);
    site_path.push(&id);
    let url = trade_url(&site_path)?.to_string();

    let listings = if ids.is_empty() {
        Vec::new()
    } else {
        let joined = ids.join(",");
        let mut fetch_path: Vec<&str> = api.to_vec();
        fetch_path.push("fetch");
        fetch_path.push(&joined);
        let mut fetch = trade_url(&fetch_path)?;
        fetch.query_pairs_mut().append_pair("query", &id);
        if let Some(realm) = realm(game) {
            fetch.query_pairs_mut().append_pair("realm", realm);
        }
        listings(&send(&state, client().get(fetch.clone()), fetch.as_str()).await?)?
    };
    Ok(TradeResult { total, url, listings })
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NinjaLine {
    name: String,
    base_type: Option<String>,
    variant: Option<String>,
    gem_level: Option<u64>,
    gem_quality: Option<u64>,
    corrupted: bool,
    links: Option<u64>,
    map_tier: Option<u64>,
    /// In the table's primary currency.
    value: f64,
    listings: Option<u64>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NinjaTable {
    /// poe.ninja's currency id that `value` is in: "chaos" (PoE1) or "divine" (PoE2).
    primary: String,
    /// How many of each other currency one `primary` is worth.
    rates: BTreeMap<String, f64>,
    lines: Vec<NinjaLine>,
}

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string)
}

/// Both of poe.ninja's shapes: exchange tables (`lines` of ids, `items` naming them) and stash
/// tables (named `lines`). Values come as `primaryValue` with a `core`, or PoE1's `chaosValue`.
fn parse_ninja(v: &Value) -> NinjaTable {
    let core = v.get("core");
    let primary = core.and_then(|c| text(c, "primary")).unwrap_or_else(|| "chaos".into());
    let mut rates: BTreeMap<String, f64> = core
        .and_then(|c| c.get("rates"))
        .and_then(Value::as_object)
        .map(|r| r.iter().filter_map(|(k, v)| Some((k.clone(), v.as_f64().filter(|v| *v > 0.0)?))).collect())
        .unwrap_or_default();
    let names: HashMap<&str, &str> = v
        .get("items")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(|i| Some((i.get("id")?.as_str()?, i.get("name")?.as_str()?))).collect())
        .unwrap_or_default();
    let mut lines = Vec::new();
    for line in v.get("lines").and_then(Value::as_array).into_iter().flatten() {
        let name = text(line, "name").or_else(|| line.get("id").and_then(Value::as_str).and_then(|id| names.get(id)).map(|n| n.to_string()));
        let value = line.get("primaryValue").or_else(|| line.get("chaosValue")).and_then(Value::as_f64);
        let (Some(name), Some(value)) = (name, value) else { continue };
        if core.is_none() && !rates.contains_key("divine") {
            // PoE1 stash tables carry both values on each line instead of a rate.
            if let Some(divine) = line.get("divineValue").and_then(Value::as_f64).filter(|d| *d > 0.0 && value > 0.0) {
                rates.insert("divine".into(), divine / value);
            }
        }
        lines.push(NinjaLine {
            name,
            base_type: text(line, "baseType"),
            variant: text(line, "variant"),
            gem_level: line.get("gemLevel").and_then(Value::as_u64),
            gem_quality: line.get("gemQuality").and_then(Value::as_u64),
            corrupted: line.get("corrupted").and_then(Value::as_bool).unwrap_or(false),
            links: line.get("links").and_then(Value::as_u64),
            map_tier: line.get("mapTier").and_then(Value::as_u64),
            value,
            listings: line.get("listingCount").and_then(Value::as_u64),
        });
    }
    NinjaTable { primary, rates, lines }
}

/// One of poe.ninja's price tables. `kind` is "exchange" (currency-like) or "stash" (items);
/// `category` is poe.ninja's own type name, such as `Currency` or `UniqueArmour`.
#[tauri::command]
pub async fn price_ninja(state: State<'_, PriceState>, game: Game, league: String, kind: String, category: String) -> Result<NinjaTable, String> {
    let path = match kind.as_str() {
        "exchange" => "exchange/current/overview",
        "stash" => "stash/current/item/overview",
        _ => return Err(format!("unknown poe.ninja table kind {kind}")),
    };
    if category.is_empty() || !category.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err(format!("unknown poe.ninja category {category}"));
    }
    let key = format!("{}/{kind}/{category}/{league}", game.id());
    if let Some((at, table)) = state.ninja.lock().unwrap().get(&key) {
        if at.elapsed() < NINJA_TTL {
            return Ok(table.clone());
        }
    }
    let mut url = reqwest::Url::parse(&format!("{NINJA}{}/api/economy/{path}", game.id())).map_err(|e| e.to_string())?;
    url.query_pairs_mut().append_pair("league", &league).append_pair("type", &category);
    let resp = client().get(url).send().await.map_err(|e| format!("poe.ninja: {e}"))?;
    match resp.status().as_u16() {
        200 => {}
        404 => return Err("poe.ninja has no prices for that league.".into()),
        429 => return Err("poe.ninja is limiting requests. Wait a minute, then try again.".into()),
        status => return Err(format!("poe.ninja returned HTTP {status}")),
    }
    let v: Value = resp.json().await.map_err(|_| "poe.ninja sent prices this app could not read".to_string())?;
    let table = parse_ninja(&v);
    state.ninja.lock().unwrap().insert(key, (Instant::now(), table.clone()));
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn exchange_tables_name_their_lines() {
        let t = parse_ninja(&json!({
            "core": { "primary": "chaos", "secondary": "divine", "rates": { "divine": 0.004 } },
            "lines": [{ "id": "divine", "primaryValue": 250.0 }, { "id": "unnamed", "primaryValue": 1.0 }],
            "items": [{ "id": "divine", "name": "Divine Orb" }]
        }));
        assert_eq!((t.primary.as_str(), t.rates.get("divine")), ("chaos", Some(&0.004)));
        assert_eq!(t.lines.len(), 1);
        assert_eq!((t.lines[0].name.as_str(), t.lines[0].value), ("Divine Orb", 250.0));
    }

    #[test]
    fn poe1_stash_tables_derive_the_divine_rate() {
        let t = parse_ninja(&json!({ "lines": [
            { "name": "Vaal Rejuvenation Totem", "variant": "20/20c", "gemLevel": 20, "gemQuality": 20, "corrupted": true, "chaosValue": 400.0, "divineValue": 2.0, "listingCount": 3 }
        ]}));
        assert_eq!(t.primary, "chaos");
        assert_eq!(t.rates.get("divine"), Some(&0.005));
        let l = &t.lines[0];
        assert_eq!((l.gem_level, l.gem_quality, l.corrupted, l.listings), (Some(20), Some(20), true, Some(3)));
    }

    #[test]
    fn poe2_stash_tables_are_in_divines() {
        let t = parse_ninja(&json!({
            "core": { "primary": "divine", "secondary": "chaos", "rates": { "chaos": 10.86 } },
            "lines": [{ "name": "Loreweave", "baseType": "Elegant Ringmail", "primaryValue": 0.92, "listingCount": 9287 }]
        }));
        assert_eq!((t.primary.as_str(), t.rates.get("chaos")), ("divine", Some(&10.86)));
        assert_eq!(t.lines[0].base_type.as_deref(), Some("Elegant Ringmail"));
    }

    #[test]
    fn listings_keep_price_age_and_seller() {
        let got = listings(
            &json!({ "result": [
                { "listing": { "indexed": "2026-11-19T18:00:00Z", "account": { "name": "Seller", "online": { "league": "Covens" } }, "price": { "type": "~price", "amount": 3, "currency": "divine" } }, "item": {} },
                { "listing": { "account": { "name": "Away", "online": null } }, "item": { "stackSize": 20 } },
                null
            ]})
            .to_string(),
        )
        .unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!((got[0].amount, got[0].currency.as_deref(), got[0].online), (Some(3.0), Some("divine"), true));
        assert_eq!((got[1].amount, got[1].online, got[1].stack), (None, false, Some(20)));
    }

    #[test]
    fn trade_urls_encode_the_league_and_stay_on_the_api() {
        let url = trade_url(&["api", "trade2", "search", "poe2", "HC Forbidden Rites"]).unwrap();
        assert_eq!(url.as_str(), "https://www.pathofexile.com/api/trade2/search/poe2/HC%20Forbidden%20Rites");
        assert!(crate::ggg::trade_policy(url.as_str()).is_ok());
        let fetch = trade_url(&["api", "trade", "fetch", "a,b"]).unwrap();
        assert!(crate::ggg::trade_policy(fetch.as_str()).is_ok());
    }

    #[test]
    fn bare_queries_are_wrapped_and_sorted_by_price() {
        let b = search_body(r#"{"status":{"option":"securable"},"type":"Two-Stone Ring"}"#).unwrap();
        assert_eq!(b["query"]["type"], "Two-Stone Ring");
        assert_eq!(b["sort"]["price"], "asc");
        let kept = search_body(r#"{"query":{"name":"Tabula Rasa"},"sort":{"indexed":"desc"}}"#).unwrap();
        assert_eq!((kept["query"]["name"].as_str(), kept["sort"]["indexed"].as_str()), (Some("Tabula Rasa"), Some("desc")));
        assert!(search_body("[1]").is_err() && search_body("nope").is_err());
    }

    #[test]
    fn trade_errors_read_ggg_messages() {
        assert_eq!(error_message(r#"{"error":{"code":2,"message":"Invalid query"}}"#).as_deref(), Some("Invalid query"));
        assert_eq!(error_message("nope"), None);
    }
}
