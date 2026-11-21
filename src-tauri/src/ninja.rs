use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::game::Game;

const HOST: &str = "https://poe.ninja/";
const UA: &str = concat!("PoE Forge/", env!("CARGO_PKG_VERSION"), " (+https://github.com/minimalguru-dev/pob-forge)");

const NO_PROFILE: &str = "poe.ninja has no profile for that account. Check the name, including its #1234 part.";
const LIMITED: &str = "poe.ninja is limiting requests. Wait a minute, then try again.";
/// Cloudflare's "Attention Required" page: poe.ninja refuses automated requests from this computer
/// for now. The app does not try to get past it.
pub const BLOCKED: &str = "poe.ninja is refusing automated requests from this computer right now (its Cloudflare protection). This usually passes; until then open poe.ninja in a browser.";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileCharacter {
    account_name: Option<String>,
    name: String,
    level: Option<u32>,
    updated: Option<String>,
    #[serde(default)]
    is_current: bool,
    league: Option<String>,
    league_url: Option<String>,
    class_name: Option<String>,
    eligibility: Option<Eligibility>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Eligibility {
    #[serde(rename = "type")]
    kind: String,
    nominal_min_level: Option<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    pub name: String,
    pub class_name: Option<String>,
    pub level: u32,
    pub league: String,
    pub league_url: String,
    pub updated: Option<String>,
    pub is_current: bool,
    pub status: String,
    pub min_level: Option<u32>,
}

#[derive(Serialize)]
pub struct CharacterList {
    pub account: String,
    pub characters: Vec<Character>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexState {
    snapshot_versions: Vec<Snapshot>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    url: String,
    #[serde(rename = "type")]
    kind: Option<String>,
    version: String,
    snapshot_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CharacterBuild {
    path_of_building_export: Option<String>,
}

fn ninja_account(name: &str) -> String {
    let name: String = name.chars().filter(|c| !c.is_whitespace()).collect();
    match name.rfind(['#', '-']) {
        Some(i) => format!("{}-{}", &name[..i], &name[i + 1..]),
        None => name,
    }
}

fn hash_account(name: &str) -> String {
    match name.rfind('-') {
        Some(i) => format!("{}#{}", &name[..i], &name[i + 1..]),
        None => name.to_string(),
    }
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().user_agent(UA).build().map_err(|e| e.to_string())
}

async fn get(client: &reqwest::Client, url: &str) -> Result<Option<String>, String> {
    let resp = client.get(url).timeout(Duration::from_secs(20)).send().await.map_err(|e| format!("poe.ninja: {e}"))?;
    let status = resp.status().as_u16();
    match status {
        200..=299 => resp.text().await.map(Some).map_err(|e| format!("poe.ninja: {e}")),
        404 => Ok(None),
        429 => Err(LIMITED.into()),
        403 => Err(BLOCKED.into()),
        _ => Err(format!("poe.ninja returned HTTP {status}")),
    }
}

/// poe.ninja caches the character list per version and matches the account name case-sensitively.
async fn profile_version(client: &reqwest::Client, game: Game, account: &str) -> Result<Option<u64>, String> {
    let url = format!("{HOST}{}/api/events/characters/{account}", game.id());
    let mut resp = client
        .get(&url)
        .header("accept", "text/event-stream")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("poe.ninja: {e}"))?;
    match resp.status().as_u16() {
        200..=299 => {}
        404 => return Ok(None),
        429 => return Err(LIMITED.into()),
        status => return Err(format!("poe.ninja returned HTTP {status}")),
    }
    // the stream stays open; only its first event is needed
    let mut buf = String::new();
    while let Some(chunk) = resp.chunk().await.map_err(|e| format!("poe.ninja: {e}"))? {
        buf.push_str(&String::from_utf8_lossy(&chunk).replace("\r\n", "\n"));
        while let Some(end) = buf.find("\n\n") {
            let event: String = buf.drain(..end + 2).collect();
            let name = event.lines().find_map(|l| l.strip_prefix("event:")).map(str::trim);
            let data: String = event.lines().filter_map(|l| l.strip_prefix("data:")).map(str::trim).collect();
            match name {
                Some("message") | None if !data.is_empty() => {
                    let version = serde_json::from_str::<serde_json::Value>(&data).ok().and_then(|v| v.get("version")?.as_u64());
                    if version.is_some() {
                        return Ok(version);
                    }
                }
                Some("message") | None => {}
                Some(_) => return Ok(None),
            }
        }
        if buf.len() > 16_000 {
            break;
        }
    }
    Ok(None)
}

pub async fn list(game: Game, account: &str) -> Result<CharacterList, String> {
    let mut account = ninja_account(account);
    if account.is_empty() {
        return Err("Enter an account name.".into());
    }
    let client = client()?;
    let mut version = profile_version(&client, game, &account).await?;
    if version.is_none() {
        if let Some(real) = crate::character::account_casing(&hash_account(&account)).await.filter(|r| *r != account) {
            version = profile_version(&client, game, &real).await?;
            account = real;
        }
    }
    let version = version.ok_or(NO_PROFILE)?;
    let body = get(&client, &format!("{HOST}{}/api/profile/characters/{account}/{version}", game.id())).await?.ok_or(NO_PROFILE)?;
    let found: Vec<ProfileCharacter> = serde_json::from_str(&body).map_err(|_| "poe.ninja sent a character list this app could not read".to_string())?;
    if found.is_empty() {
        return Err("That poe.ninja profile has no characters for this game.".into());
    }
    let account = found.iter().find_map(|c| c.account_name.clone()).unwrap_or_else(|| hash_account(&account));
    let characters = found
        .into_iter()
        .map(|c| Character {
            status: c.eligibility.as_ref().map_or_else(|| "unlisted".into(), |e| e.kind.clone()),
            min_level: c.eligibility.as_ref().and_then(|e| e.nominal_min_level),
            name: c.name,
            class_name: c.class_name,
            level: c.level.unwrap_or(0),
            league: c.league.unwrap_or_default(),
            league_url: c.league_url.unwrap_or_default(),
            updated: c.updated,
            is_current: c.is_current,
        })
        .collect();
    Ok(CharacterList { account, characters })
}

pub async fn build_code(game: Game, account: &str, character: &str, league_url: &str) -> Result<String, String> {
    const NO_BUILD: &str = "poe.ninja has no build for that character.";
    let client = client()?;
    let index = get(&client, &format!("{HOST}{}/api/data/index-state", game.id())).await?.ok_or("poe.ninja did not return its league list")?;
    let index: IndexState = serde_json::from_str(&index).map_err(|_| "poe.ninja sent a league list this app could not read".to_string())?;
    // PoE1 leagues have an experience ladder and a delve depth ladder; both hold the same characters
    let snapshot = index
        .snapshot_versions
        .iter()
        .filter(|s| s.url == league_url)
        .min_by_key(|s| !matches!(s.kind.as_deref(), None | Some("exp")))
        .ok_or("poe.ninja no longer has build data for that league.")?;
    let q = crate::character::query_value;
    let url = format!(
        "{HOST}{}/api/builds/{}/character?account={}&name={}&overview={}&timeMachine=",
        game.id(),
        q(&snapshot.version),
        q(&ninja_account(account)),
        q(character),
        q(&snapshot.snapshot_name),
    );
    let body = get(&client, &url).await?.ok_or(NO_BUILD)?;
    let build: CharacterBuild = serde_json::from_str(&body).map_err(|_| "poe.ninja sent a character this app could not read".to_string())?;
    build.path_of_building_export.filter(|c| !c.trim().is_empty()).ok_or_else(|| NO_BUILD.into())
}

// ---------------------------------------------------------------------------
// Build search: the ladder poe.ninja's builds page shows, for the assistant to compare against.
// The search answers in protobuf; text columns point into dictionaries ("NDIC") fetched by hash.
// ---------------------------------------------------------------------------

/// A protobuf field: its number and either a varint or a length-delimited payload.
enum PbVal<'a> {
    Int(u64),
    Bytes(&'a [u8]),
    Fixed,
}

fn pb_varint(b: &[u8], i: &mut usize) -> Option<u64> {
    let (mut r, mut s) = (0u64, 0u32);
    loop {
        let x = *b.get(*i)?;
        *i += 1;
        r |= u64::from(x & 0x7f).checked_shl(s)?;
        if x & 0x80 == 0 {
            return Some(r);
        }
        s += 7;
    }
}

fn pb_fields(b: &[u8]) -> Option<Vec<(u32, PbVal<'_>)>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let key = pb_varint(b, &mut i)?;
        let field = u32::try_from(key >> 3).ok()?;
        match key & 7 {
            0 => out.push((field, PbVal::Int(pb_varint(b, &mut i)?))),
            2 => {
                let len = usize::try_from(pb_varint(b, &mut i)?).ok()?;
                let end = i.checked_add(len).filter(|e| *e <= b.len())?;
                out.push((field, PbVal::Bytes(&b[i..end])));
                i = end;
            }
            1 => {
                i += 8;
                out.push((field, PbVal::Fixed));
            }
            5 => {
                i += 4;
                out.push((field, PbVal::Fixed));
            }
            _ => return None,
        }
    }
    (i == b.len()).then_some(out)
}

fn pb_packed(b: &[u8]) -> Vec<u64> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match pb_varint(b, &mut i) {
            Some(v) => out.push(v),
            None => break,
        }
    }
    out
}

fn pb_str(v: &PbVal) -> String {
    match v {
        PbVal::Bytes(b) => String::from_utf8_lossy(b).into_owned(),
        _ => String::new(),
    }
}

/// One result column: plain strings, numbers, or indexes (lists of indexes) into a dictionary.
#[derive(Default)]
struct Column {
    name: String,
    dictionary: Option<String>,
    strings: Vec<String>,
    numbers: Vec<u64>,
    lists: Vec<Vec<u64>>,
}

struct Search {
    total: u64,
    /// Dictionary name to its hash.
    dictionaries: Vec<(String, String)>,
    columns: Vec<Column>,
}

fn parse_search(b: &[u8]) -> Option<Search> {
    let top = pb_fields(b)?;
    let PbVal::Bytes(body) = top.iter().find(|(f, _)| *f == 1)?.1 else { return None };
    let mut s = Search { total: 0, dictionaries: Vec::new(), columns: Vec::new() };
    for (f, v) in pb_fields(body)? {
        match (f, v) {
            (1, PbVal::Int(n)) => s.total = n,
            (6, PbVal::Bytes(d)) => {
                let d = pb_fields(d)?;
                let name = d.iter().find(|(f, _)| *f == 1).map(|(_, v)| pb_str(v)).unwrap_or_default();
                // The first hash is the names; later ones carry icons and the like.
                if let Some(hash) = d.iter().find(|(f, _)| *f == 2).map(|(_, v)| pb_str(v)) {
                    s.dictionaries.push((name, hash));
                }
            }
            (12, PbVal::Bytes(c)) => {
                let mut col = Column::default();
                for (f, v) in pb_fields(c)? {
                    match (f, v) {
                        (1, v) => col.name = pb_str(&v),
                        (11, v) => col.dictionary = Some(pb_str(&v)),
                        (7, v) => col.strings.push(pb_str(&v)),
                        (6, PbVal::Bytes(p)) => col.numbers.extend(pb_packed(p)),
                        (9, PbVal::Bytes(l)) => {
                            let ids = pb_fields(l)?.into_iter().find(|(f, _)| *f == 1).map(|(_, v)| match v {
                                PbVal::Bytes(p) => pb_packed(p),
                                PbVal::Int(n) => vec![n],
                                PbVal::Fixed => Vec::new(),
                            });
                            col.lists.push(ids.unwrap_or_default());
                        }
                        _ => {}
                    }
                }
                s.columns.push(col);
            }
            _ => {}
        }
    }
    Some(s)
}

/// poe.ninja's dictionary format: "NDIC", version, count, a sparse index, one length byte per word, the words.
fn parse_dictionary(b: &[u8]) -> Option<Vec<String>> {
    let u32_at = |at: usize| b.get(at..at + 4).map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]) as usize);
    if b.get(0..4)? != b"NDIC" {
        return None;
    }
    let count = u32_at(12)?;
    let index = u32_at(28)?;
    let mut at = 36usize.checked_add(index.checked_mul(8)?)?;
    let lengths = b.get(at..at.checked_add(count)?)?;
    at += count;
    let mut out = Vec::with_capacity(count);
    for &len in lengths {
        let end = at + usize::from(len);
        out.push(String::from_utf8_lossy(b.get(at..end)?).into_owned());
        at = end;
    }
    Some(out)
}

static DICTIONARIES: std::sync::Mutex<Option<std::collections::HashMap<String, std::sync::Arc<Vec<String>>>>> = std::sync::Mutex::new(None);

async fn get_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let resp = client.get(url).timeout(Duration::from_secs(25)).send().await.map_err(|e| format!("poe.ninja: {e}"))?;
    match resp.status().as_u16() {
        200..=299 => resp.bytes().await.map(|b| b.to_vec()).map_err(|e| format!("poe.ninja: {e}")),
        429 => Err(LIMITED.into()),
        403 => Err(BLOCKED.into()),
        s => Err(format!("poe.ninja returned HTTP {s}")),
    }
}

async fn dictionary(client: &reqwest::Client, game: Game, hash: &str) -> Result<std::sync::Arc<Vec<String>>, String> {
    if let Some(d) = DICTIONARIES.lock().unwrap_or_else(|e| e.into_inner()).as_ref().and_then(|m| m.get(hash)).cloned() {
        return Ok(d);
    }
    let body = get_bytes(client, &format!("{HOST}{}/api/builds/dictionary/{hash}", game.id())).await?;
    let words = std::sync::Arc::new(parse_dictionary(&body).ok_or("poe.ninja sent a dictionary this app could not read")?);
    DICTIONARIES.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(Default::default).insert(hash.to_string(), words.clone());
    Ok(words)
}

/// One ladder character the search found.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LadderBuild {
    pub rank: usize,
    pub name: String,
    pub account: String,
    pub class: Option<String>,
    pub level: Option<u64>,
    pub life: Option<u64>,
    pub energy_shield: Option<u64>,
    pub ehp: Option<String>,
    pub dps: Option<String>,
    pub skills: Vec<String>,
    pub keystones: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LadderSearch {
    pub league: String,
    /// What compare_ninja_build passes back to find a character's build.
    pub league_url: String,
    pub total: u64,
    pub url: String,
    pub builds: Vec<LadderBuild>,
}

/// The ladder snapshot for a league as the user names it, or the current challenge league.
async fn ladder_snapshot(client: &reqwest::Client, game: Game, league: Option<&str>) -> Result<(Snapshot, String), String> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Named {
        url: String,
        #[serde(rename = "type")]
        kind: Option<String>,
        name: String,
        version: String,
        snapshot_name: String,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Index {
        snapshot_versions: Vec<Named>,
    }
    static INDEX: std::sync::Mutex<Option<(std::time::Instant, Game, String)>> = std::sync::Mutex::new(None);
    let cached = INDEX.lock().unwrap_or_else(|e| e.into_inner()).as_ref().filter(|(at, g, _)| *g == game && at.elapsed() < Duration::from_secs(600)).map(|(_, _, b)| b.clone());
    let body = match cached {
        Some(b) => b,
        None => {
            let b = get(client, &format!("{HOST}{}/api/data/index-state", game.id())).await?.ok_or("poe.ninja did not return its league list")?;
            *INDEX.lock().unwrap_or_else(|e| e.into_inner()) = Some((std::time::Instant::now(), game, b.clone()));
            b
        }
    };
    let index: Index = serde_json::from_str(&body).map_err(|_| "poe.ninja sent a league list this app could not read".to_string())?;
    let ladders = index.snapshot_versions.into_iter().filter(|s| matches!(s.kind.as_deref(), None | Some("exp")));
    let wanted = league.map(|l| l.trim().to_lowercase()).filter(|l| !l.is_empty());
    let found = match &wanted {
        Some(l) => ladders.into_iter().find(|s| s.name.to_lowercase() == *l || s.url.to_lowercase() == *l),
        None => ladders.into_iter().next(),
    };
    let s = found.ok_or_else(|| format!("poe.ninja has no ladder for {}", league.unwrap_or("the current league")))?;
    Ok((Snapshot { url: s.url, kind: s.kind, version: s.version, snapshot_name: s.snapshot_name }, s.name))
}

/// The top characters poe.ninja's builds page lists for these filters, best first.
pub async fn search_builds(
    game: Game,
    league: Option<&str>,
    class: Option<&str>,
    skill: Option<&str>,
    keystone: Option<&str>,
    item: Option<&str>,
    limit: usize,
) -> Result<LadderSearch, String> {
    let client = client()?;
    let (snapshot, league_name) = ladder_snapshot(&client, game, league).await?;
    let mut filters: Vec<(&str, &str)> = Vec::new();
    for (key, value) in [("class", class), ("skills", skill), ("keypassives", keystone), ("items", item)] {
        if let Some(v) = value.map(str::trim).filter(|v| !v.is_empty()) {
            filters.push((key, v));
        }
    }
    let mut params: Vec<(&str, &str)> = vec![("overview", snapshot.snapshot_name.as_str()), ("type", "exp")];
    params.extend(filters.iter().copied());
    let api = reqwest::Url::parse_with_params(&format!("{HOST}{}/api/builds/{}/search", game.id(), snapshot.version), &params).map_err(|e| e.to_string())?;
    let page = reqwest::Url::parse_with_params(&format!("{HOST}{}/builds/{}", game.id(), snapshot.url), &filters).map_err(|e| e.to_string())?;
    // A refused search still gives the user the page to open themselves.
    let body = get_bytes(&client, api.as_str()).await.map_err(|e| if e == BLOCKED { format!("{BLOCKED} The same search: {page}") } else { e })?;
    let search = parse_search(&body).ok_or("poe.ninja sent a build search this app could not read")?;

    let mut words: std::collections::HashMap<String, std::sync::Arc<Vec<String>>> = std::collections::HashMap::new();
    for (name, hash) in &search.dictionaries {
        if search.columns.iter().any(|c| c.dictionary.as_deref() == Some(name.as_str())) {
            words.insert(name.clone(), dictionary(&client, game, hash).await?);
        }
    }
    let column = |name: &str| search.columns.iter().find(|c| c.name == name);
    let word = |col: &Column, id: u64| -> Option<String> {
        let dict = words.get(col.dictionary.as_deref()?)?;
        dict.get(usize::try_from(id).ok()?).cloned()
    };
    let number = |name: &str, row: usize| column(name).and_then(|c| c.numbers.get(row).copied());
    let text = |c: Option<&Column>, row: usize| c.and_then(|c| c.strings.get(row).cloned()).filter(|s| !s.is_empty());
    let list = |name: &str, row: usize, max: usize| -> Vec<String> {
        column(name)
            .and_then(|c| c.lists.get(row).map(|ids| ids.iter().filter_map(|id| word(c, *id)).take(max).collect()))
            .unwrap_or_default()
    };
    let dps = search.columns.iter().find(|c| c.name.starts_with("dps") && c.name.ends_with(".total"));
    let rows = column("name").map_or(0, |c| c.strings.len());
    let builds = (0..rows.min(limit))
        .map(|row| LadderBuild {
            rank: row + 1,
            name: text(column("name"), row).unwrap_or_default(),
            account: text(column("account"), row).unwrap_or_default(),
            class: column("class").and_then(|c| c.numbers.get(row).and_then(|id| word(c, *id))),
            level: number("level", row),
            life: number("life", row),
            energy_shield: number("energyshield", row),
            ehp: text(column("ehp__str"), row),
            dps: text(dps, row),
            skills: list("skills", row, 4),
            keystones: list("keypassives", row, 8),
        })
        .collect();
    Ok(LadderSearch { league: league_name, league_url: snapshot.url, total: search.total, url: page.to_string(), builds })
}

#[cfg(test)]
mod tests {
    use super::{hash_account, ninja_account, parse_dictionary, parse_search, search_builds};
    use crate::game::Game;

    fn varint(mut v: u64, out: &mut Vec<u8>) {
        loop {
            let b = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                out.push(b);
                return;
            }
            out.push(b | 0x80);
        }
    }
    fn field(n: u32, payload: &[u8], out: &mut Vec<u8>) {
        varint(u64::from(n) << 3 | 2, out);
        varint(payload.len() as u64, out);
        out.extend_from_slice(payload);
    }

    #[test]
    fn dictionaries_decode() {
        let words = ["Duelist", "Slayer"];
        let mut b = b"NDIC".to_vec();
        for w in [2u32, 0, 2, 0, 0, 16, 1, 2, 0, 0] {
            b.extend_from_slice(&w.to_le_bytes());
        }
        // header is 36 bytes, then one 8-byte index entry
        b.truncate(36 + 8);
        b.extend(words.iter().map(|w| w.len() as u8));
        for w in words {
            b.extend_from_slice(w.as_bytes());
        }
        assert_eq!(parse_dictionary(&b).unwrap(), words);
        assert_eq!(parse_dictionary(b"nope"), None);
    }

    #[test]
    fn search_columns_decode() {
        let mut name = Vec::new();
        field(1, b"name", &mut name);
        field(7, b"Alpha", &mut name);
        field(7, b"Beta", &mut name);
        let mut class = Vec::new();
        field(1, b"class", &mut class);
        field(11, b"class", &mut class);
        field(6, &[1, 0], &mut class);
        let mut list = Vec::new();
        field(1, &[5, 3], &mut list);
        let mut skills = Vec::new();
        field(1, b"skills", &mut skills);
        field(11, b"gem", &mut skills);
        field(9, &list, &mut skills);
        let mut dict = Vec::new();
        field(1, b"class", &mut dict);
        field(2, b"abc", &mut dict);
        let mut body = vec![0x08, 124];
        field(6, &dict, &mut body);
        field(12, &name, &mut body);
        field(12, &class, &mut body);
        field(12, &skills, &mut body);
        let mut top = Vec::new();
        field(1, &body, &mut top);
        let s = parse_search(&top).unwrap();
        assert_eq!(s.total, 124);
        assert_eq!(s.dictionaries, vec![("class".to_string(), "abc".to_string())]);
        assert_eq!(s.columns[0].strings, ["Alpha", "Beta"]);
        assert_eq!(s.columns[1].numbers, [1, 0]);
        assert_eq!(s.columns[2].lists, vec![vec![5, 3]]);
    }

    /// Talks to poe.ninja: `cargo test -p pob-forge --lib ninja -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn live_ladder_search() {
        pob_engine::install_tls_provider();
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let r = rt.block_on(search_builds(Game::Poe1, None, Some("Slayer"), Some("Dual Strike of Ambidexterity"), None, None, 5)).unwrap();
        assert!(r.total > 0 && !r.builds.is_empty(), "{}", r.total);
        let first = &r.builds[0];
        assert_eq!(first.class.as_deref(), Some("Slayer"));
        assert!(first.skills.iter().any(|s| s == "Dual Strike of Ambidexterity"), "{:?}", first.skills);
        println!("{} {} of {}: {} ({}) level {:?} dps {:?} keystones {:?}", r.league, r.builds.len(), r.total, first.name, first.account, first.level, first.dps, first.keystones);
    }

    #[test]
    fn account_names() {
        assert_eq!(ninja_account(" ohitsjudd#7248 "), "ohitsjudd-7248");
        assert_eq!(ninja_account("ohitsjudd-7248"), "ohitsjudd-7248");
        assert_eq!(ninja_account("some-name#12"), "some-name-12");
        assert_eq!(hash_account("some-name-12"), "some-name#12");
    }
}
