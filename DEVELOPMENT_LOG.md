# Development log — PoB Redux Progression

Ovaj dokument je aktualni pregled za sljedećeg razvojnog agenta. Dopunjavati pri važnoj odluci, ispravku, commitu i završenoj validaciji. Ne tvrditi da je build ili runtime test prošao bez rezultata. Za pogreške i prevenciju vidi [mistakes.md](mistakes.md); za ponašanje proizvoda vidi [PROGRESSION.md](PROGRESSION.md).

## Projekt i autorizirani opseg

- Korisnik je autorizirao autonoman razvoj forka za PoE1: Progression tab, current/target import, lokalni snapshot history, equipment/passive diff, readiness i prioritete, osnovni bossing profil, testove, dokumentaciju, Windows validation build i PR ako je moguć.
- Ne čitati memoriju igre, ne tražiti session cookies, ne automatizirati gameplay. Izračuni ostaju u postojećem PoB Lua engineu.
- 2026-11-15 dodatno je zatraženo kontinuirano vođenje `mistakes.md` i development loga radi preuzimanja rada od drugih AI agenata.
- [Upstream](https://github.com/pobredux/pob-redux), [fork](https://github.com/minimalguru-dev/pob-redux), [PR #1](https://github.com/minimalguru-dev/pob-redux/pull/1).
- Grana: `feat/poe1-progression`; upstream početna točka: `240b09bab1d73363afdf0c1c715dbf5c6e0cb811`.
- `upstream` remote je `pobredux/pob-redux`; `origin` je `minimalguru-dev/pob-redux`. PR je otvoren prema `main` vlastitog forka, bez mergea ili produkcijskog releasea.
- MIT licenca je pročitana i ostavljena netaknuta. PoB pinovi ostaju u `pob-sync.toml`: PoE1 `f5ca0aeeeeba62d7d2c93b1fbd0704c74d54e7ea`, PoE2 `2450f2fc6ff5d35ace9b35bcd9d31e648ae9d7f1`.

## Implementirano

1. `crates/pob-engine/lua/progression.lua`: novi `progression_capture`, privremeni upstream `CompareEntry`, snapshot i izolirani boss scenario; registracija kroz `engine.rs` i završetak `bridge.lua`.
2. `src/lib/progression/model.ts`: snapshot schema v1, raw-item/variant diff, node/mastery diff, readiness provjere, deterministički prioriteti, validacija historije i kontrolirano lokalno spremanje.
3. `src/lib/views/ProgressionView.svelte`: PoE1 tab, uvoz code/XML/URL ili otvorenog builda, current/target selekcija, lokalna povijest, backup import/export, diffovi, provjere i PoB statistike.
4. Navigacija: `App.svelte`, `TitleBar.svelte`, `CommandPalette.svelte`, `ViewId` i engleski Paraglide ključ.
5. Windows validation workflow i zasebni preview identity/URL scheme. Preview isključuje upstream update provjere preko `VITE_PROGRESSION_VALIDATION`.
6. Dokumentacija i testovi navedeni niže.

## Važne odluke i granice

- Capture ne učitava target kao live build i ne mijenja Compare listu. Jedan engine call izbjegava frontend niz privremenih mutacija/rollbackova.
- Snapshot sadrži originalni XML i rezultate u trenutku snimanja. Promjene live builda ne osvježavaju povijest automatski.
- LocalStorage ključ: `pob-redux:progression:poe1:v1`; limit 20 snapshota / 4 MiB; nema automatskog izbacivanja starih zapisa. Storage failure ne smije objaviti nespremljeni plan kao spremljen.
- Naziv itema nije dovoljan za jednakost: uspoređuje se raw tekst i odabrane varijante. Različite verzije stabla/klase ne uspoređuju raw node ID-ove.
- Status ne prikazuje izmišljeni readiness postotak niti automatski odobrava switch. Ownership, refund budget, stvarni sustain i cijene ostaju manual review.
- PoB CompareEntry radi solo; ne učitava party buffove. Boss profil isključuje deklarirane charge/buff/kill/flask pretpostavke, ali ostavlja ostali importirani config. To nije izmjereni combat DPS niti potpuna simulacija održivosti.
- MCP registry, postojeći calculation engine i OAuth ponašanje nisu mijenjani. Novi bridge method nije dodan javnom MCP tool registryju.
- Progression tekstovi su zasad engleski; label taba koristi postojeći localization sustav.

## Commitovi

| Commit | Sadržaj |
|---|---|
| `bb74bd1` | PoE1 Progression, Lua capture/scenario, model, UI, unit/engine testovi, Windows workflow |
| `fcc2efa` | Ispravni upstream theme tokeni i textarea markup |
| `61ebd64` | Browser testovi, guide, item priorities, Windows environment ispravak |
| `b84e2cc` | Development log i mistakes status, samo dokumentacija |
| `40df7a6` | LF patch pravilo i stroža validacija backupova |
| `3eefd01` | Workspace-relative native test path i provjera flask scenarija |
| `90cd6bb` | Unicode snapshot nazivi i zabilježena real-engine validacija |

## Validacija — potvrđeno lokalno

- Bun 1.4.4 i dependency install iz lockfilea rade.
- `bun test scripts`: **25 pass, 0 fail**, uključuje 15 novih Progression unit testova i 10 postojećih testova; zadnji prolaz uključuje strožu validaciju backupova.
- `bun run check`: **0 errors, 0 warnings** nakon markup ispravka. Ponovljeni check nakon commita `61ebd64` također je završio s 0 errors / 0 warnings.
- `bun run build`: uspješan produkcijski frontend bundle; ostaje Vite upozorenje za veliki bundle. To nije potvrda native Windows builda.
- Playwright component-flow testovi: **3 pass** na lokalnom Edgeu; uvoz/greška, current/target i reload/delete, očuvanje oštećene povijesti. Ovo su IPC fixture testovi, ne PoB engine testovi.
- UI pregledan kroz renderirane screenshotove, bez horizontalnog overflowa na 1100 px; nema page-error događaja.

## Native CI — završeno uspješno

- Prvi dovršeni Windows pokušaj [43715258887](https://github.com/minimalguru-dev/pob-redux/actions/runs/43715258887) prošao je frontend check/test/build, ali pao u LuaJIT buildu zbog `NoDefaultCurrentDirectoryInExePath` / `minilua`. Detalji i ispravak su u mistakes.md.
- [Windows run 43716010378](https://github.com/minimalguru-dev/pob-redux/actions/runs/43716010378) potvrdio je LuaJIT ispravak i browser testove. Pao je zatim na CRLF patchu; uzrok lokalno reproduciran i ispravljen pravilom `*.patch text eol=lf`. Sync oba pina zatim je prošao u runu `43717323768`.
- [Linux check 43716041218](https://github.com/minimalguru-dev/pob-redux/actions/runs/43716041218) uspješno je dovršio cijeli dostupni upstream check/test pipeline za raniji commit `61ebd64`.
- Zadnji code commit `90cd6bb8e60cf68cbfb9c7012d524c66c7bddf07`: [Windows 43718149574](https://github.com/minimalguru-dev/pob-redux/actions/runs/43718149574), [Linux 43718155297](https://github.com/minimalguru-dev/pob-redux/actions/runs/43718155297).
- Zadnji Linux run završio je **success**: workspace check, 56 app testova, 24 standardna engine testa i zasebni real-engine Progression test, uključujući Unicode round-trip. Postojeći voice test ostaje ignored; Progression test koji je ignored u standardnom suiteu eksplicitno je uspješno izvršen u zasebnom koraku.
- Završni Windows run je **success**: frontend check, 25 unit testova, 3 browser testa, frontend build, sync oba pina, 24 standardna engine testa, zasebni real-engine Progression test i 59 app testova (postojeći voice test ignored). Release build i NSIS installer uspješno su završeni.
- [Installer artifact 14164402860](https://github.com/minimalguru-dev/pob-redux/actions/runs/43718149574/artifacts/14164402860): originalni naziv `PoB Redux Progression Preview_0.5.8_x64-setup.exe`, 52.715.716 bajtova; preuzet lokalno. SHA-256 samog EXE-a: `d85258d4d072110dcf9fb9c6d88d980ecb8efbaa63e5ec1dedeb0a8c8af6bed7`. GitHub artifact istječe 2026-11-29; lokalna isporuka ostaje u outputs.
- `crates/pob-engine/tests/progression.rs` uključuje Lua compile test i ignored real-engine test koji workflow eksplicitno pokreće sa synced PoE1 podacima. Provjerava parity, non-mutation, postojeći Compare state, invalid import i scenario jednakost direktnom PoB configu.
- Privatni upstream `pobredux/corpus` i `CORPUS_DEPLOY_KEY` nisu dostupni; puni golden suite nije verificiran.
- Lokalni Windows host nema Rust/MSVC/CMake preduvjete. Ne prijavljivati lokalno izgrađen native binary.

## Reprodukcija

Standardno: `bun install --frozen-lockfile`, `bun run check`, `bun test scripts`, `bun run build`.

Browser: `bun x playwright install chromium`, zatim `bun run test:progression:ui`. Lokalno je upotrijebljen `PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH=C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe`.

Native: slijediti CONTRIBUTING.md i Windows workflow. Nakon synca postaviti `POB_REDUX_POB_ROOT=src-tauri/resources/pob1`; pokrenuti `cargo test -p pob-engine --test progression -- --ignored --nocapture`. Za svaki Windows build proces ukloniti `NoDefaultCurrentDirectoryInExePath`, ne postaviti ga na prazan string.

## Sljedeći koraci pri preuzimanju

1. Ovaj minimalni razvojni opseg je dovršen; provjeriti aktualni `git status`, granu i ovaj log prije novog rada. Ne restartati završene provjere bez promjene koda ili novog razloga.
2. Validirani code commit je `90cd6bb`; završni handoff commit mijenja samo Markdown dokumentaciju i dodaje AGENTS.md. Installer odgovara code commitu, ne naknadnoj dokumentaciji.
3. Nisu verificirani interaktivna native desktop sesija, konkretni korisnikov build ni privatni golden corpus. To nisu prolazni rezultati. Buduća provjera stvarnog builda mora koristiti korisnikov stvarni input, bez izmišljanja DPS-a.
4. Isporuka uključuje Windows installer, source ZIP, SHA-256 manifest i izvještaj na hrvatskom. PR je u vlastitom forku; nema mergea niti produkcijskog releasea.
5. Pri novom razvoju ažurirati ovaj log i mistakes.md na važnim nalazima; zadržati razliku između fixture UI testova, real-engine testova i native GUI provjere.

## Dnevnik nastavka — 2026-11-15

- Na korisnikov zahtjev uvedeni mistakes.md i ovaj development log; commit `b84e2cc` mijenja samo dokumentaciju i nosi `[skip ci]` kako ne bi prekidao već pokrenutu kompilaciju nepromijenjenog koda.
- Korisnik je potvrdio nastavak. Rad se nastavlja unutar istog opsega, bez novih autorizacijskih pitanja.
- Završni pregled pooštrio je validaciju backupova (unique node IDs/slots, scalar tipovi, datum i loadout). Nakon promjene prolazi 25 unit + 3 browser testa.
- U Windows pipelineu razlikovana su dva potvrđena uzroka: minilua environment (riješen) i CRLF patch (lokalno dokazano, ispravak spreman za CI).
- Linux workflow dobiva eksplicitni real-engine Progression test prije pune workspace kompilacije, da runtime regresija bude vidljiva i na Linuxu.
- Commit `40df7a6` sadrži CRLF pravilo i backup validaciju; lokalni Svelte check 0/0 i frontend build ponovno prolaze.
- Linux run `43716753722` otkrio je cwd grešku novog test harnessa prije boota enginea. Commit `3eefd01` razrješava relativni PoB path iz workspacea; dodatno testira aktivni Ruby Flask i njegovo isključenje u boss scenariju.
- **Potvrđen runtime:** Linux run [43717329626](https://github.com/minimalguru-dev/pob-redux/actions/runs/43717329626), korak `Validate Progression against real PoB1`, završio je uspješno. Prošli su configured-stat parity, boss parity s direktnim PoB configom i isključenim flaskom, non-mutation, očuvanje Compare stanja, invalid import i ponovljeni XML import.
- Windows run `43717323768` prošao je sync oba pina nakon LF ispravka i prešao na Rust engine regresije. Time su oba ranija Windows build-preparation uzroka potvrđeno otklonjena.
- Završni code review uklonio je byte-based rezanje Unicode naziva. Dodan je točan round-trip assert za `A` + 159 znakova `č`; zadnji code commit mora ponovno proći CI. Nakon toga ne proširivati opseg.
- MCP kod dodatno pregledan: `src-tauri/src/mcp.rs` bind na `127.0.0.1`, Bearer provjera. Nije pokrenut javni server niti promijenjena autentikacija.
- Završna validacija `90cd6bb`: Linux `43718155297` i Windows `43718149574` uspješni; Windows installer preuzet i hashiran. Kod je ostao zamrznut nakon zadnjeg funkcionalnog ispravka. Završni handoff update je samo dokumentacija (`[skip ci]`).

## Naknadni auth pregled — 2026-11-15

- Korisnik pita je li sigurno autorizirati PoE i OpenAI račun. Pregledani su samo source code i službene javne upute, bez čitanja stvarnih tokena ili prijave.
- PoE: službeni HTTPS OAuth endpoints, PKCE S256, state provjera i loopback callback; tokeni se spremaju u Windows Credential Manager ili DPAPI fallback. Scopeovi: profile, characters, trade.
- Otvoreno: upstream OAuth client `pobredux` i zajednički keyring SERVICE nisu izolirani novim Tauri identifierom. Ne tvrditi da je fork zasebno odobren od GGG-a. Detalji u mistakes.md.
- OpenAI/Codex: vendor CLI login i app-server; chat thread traži read-only sandbox te isključuje shell, web i dodatne integracije. To ne izolira lokalni auth profil niti predstavlja OS izolaciju same desktop aplikacije. Razgovor i pročitani build podaci mogu biti poslani odabranom AI provideru.
- Preporuka: za test Progressiona računi nisu potrebni; koristiti PoB code/XML. Prije glavnih računa odvojiti credential store/Codex profil i razriješiti pravo korištenja OAuth clienta. Ovo je preporuka iz source reviewa, ne potvrđena kompromitacija.
- Installer i aplikacijski kod ostaju na `90cd6bb`; ova dopuna mijenja samo dokumentaciju. Službeni izvori: https://www.pathofexile.com/developer/docs i https://learn.chatgpt.com/docs/auth .

## Popravak prijavljenog Import/tree-art problema i auth audit — 2026-11-15

- Korisnik prijavljuje nefunkcionalan Paste/Import gumb i tree art download; traži sigurnosni audit autorizacije. Potvrđena je autorizacija za ispravke i nastavak, bez prijavljivanja na korisnikove račune.
- Lokalni log potvrđuje greške zapisa cachea, ne HTTP grešku. Vraćena je 81 nedostajuća art datoteka; provjereno svih 990 manifest entries za oba enginea, bez hash razlika. Originalni I/O uzrok nije sačuvan starim kodom; mali slobodni prostor na C: nije dokaz uzroka.
- Implementirano: inline Import feedback/retry, zaštita aktivnih cache temp datoteka i detaljni I/O error; credential namespace prema stvarnom Tauri app identifieru; preview PoE OAuth onemogućen u backendu i UI-ju; Codex dobiva poseban profil i obvezni keyring, bez naslijeđenih OpenAI API ključeva/proxy postavki. Token endpoint client ne prati HTTP redirecte.
- Frontend provjeren: 28 unit testova, 6 browser testova (uključujući actual ImportView clicks s IPC fixtureom), Svelte 0/0 i production bundle pass.
- Audit je pregledao vlastiti kod i javne dependency verzije; nijedan stvarni korisnikov token/password/auth.json nije čitan. Zakrpljeni undici 7.30.1 i zod 4.6.6; ostaje low dev-only deepmerge bez objavljene zakrpe.
- Cargo lockfile generiran pravim `cargo update` u izoliranoj pomoćnoj grani `chore/progression-security-lockfile`, CI 43723959255. Zakrpljeni rmcp 3.9.0, rustix 1.1.6, rustls-webpki 0.103.16, webpki-root-certs 1.0.10. Promjene Cargo resolverskih veza (windows-sys) nisu ručno prepisivane; native CI koristi --locked.
- OSV scan pokriva 657 registry paketa. Nakon zakrpa ostaju glib 0.18.5 (Linux GTK), proc-macro-error 1.0.4 te unmaintained zbus_names/zvariant/zvariant_derive. Ne tvrditi da je cijeli dependency audit čist. Nije provedena potpuna neovisna penetracijska provjera niti login glavnim računima.
- Desktop automation nije dostupna (kernel assets OS error 3). Automatski approval review odbio je launch s loopback remote-debugging priključkom; razlog nije detaljnije naveden. Taj pristup nije ponavljan niti zaobiđen.
- Sljedeće: pokrenuti i dovršiti Windows/Linux CI za novi code commit, ispraviti stvarne padove, preuzeti installer, isporučiti audit nalaz s preciznim ograničenjima. Stari installer ne sadrži ove auth ispravke.
- Follow-up review uses single-quoted TOML for the Codex keyring override so the existing Windows terminal wrapper preserves it. PowerShell argument round trip passed without login; native test extended. Documentation now explicitly separates old and new auth behavior.
- Linux run 43724475298 caught one missed shared write_atomic caller in threads.rs (E0308). Updated it to propagate the OS error; repository-wide Rust reference search completed. Fresh native validation required for this follow-up.

## Nastavak — 2026-11-16

- Provjeren završetak prethodnih runova: native kompilacija i stvarni PoE1 test prolaze, ali novi OAuth redirect test pada na obje platforme zbog nedostajuće TLS inicijalizacije u test harnessu. Windows: 62 passed, 1 failed, 1 ignored; Linux: 59 passed, 1 failed, 1 ignored. To nisu uspješni završni runovi.
- Test sada koristi isti pob_engine::install_tls_provider() kao production run(). Nema promjene TLS sigurnosne politike, ignoriranja testa ili dodatnih auth zahtjeva. Novi CI treba završiti prije izrade/isporuke installera.

## Companion overlay i Progression alati — 2026-11-16

- Lokalni razvoj prebačen na `D:\dev` (C: je bio pun). Rust 1.100, Bun 1.4.4, CMake 4.5 i VS 2022 Build Tools (MSVC 14.44, libclang) instalirani na D:; Windows SDK 10.0.26100 je na C:. `bun run sync` i native build sada prolaze lokalno.
- Lokalni klon prvo je imao CRLF patch datoteke jer je `*.patch eol=lf` stigao tek s PR granom; ponovni checkout `patches/` riješio je `git apply`. Svježi klon ove grane nema taj problem.
- Grana `feat/companion-overlay` (iz `feat/poe1-progression`), ništa nije pushano.
- Progression povijest premještena iz webview localStoragea u `progression-poe1.json` (app local data); diff, readiness i prioriteti portani u `src-tauri/src/progression.rs` s Rust testovima. Stari localStorage ključ migrira se jednom i ostaje kao backup. Asistent/MCP dobiva `get_progression`, `capture_progression_snapshot`, `select_progression_snapshots` (PoE1, 83 → 86 alata).
- Potvrđen i ispravljen postojeći bug: PoB ne stavlja `FireResistMax` itd. u output, pa su snapshotovi uvijek imali `null` cap i svaka res provjera bila "unknown". Sada se cap čita iz modDB-a kao u CalcDefence; real-engine test to provjerava (Ruby Flask daje 80 % fire cap, boss scenarij 75 %).
- Nove bridge metode: `item_evaluate` (tekst iz igre → delta po slotu, ne mijenja build; test uspoređuje sa stvarnim equipom na PoE1 i PoE2), `map_evaluate` (PoB map modovi na privremenom CompareEntry + pravila za reflect/regen/leech), `leveling_info`.
- Overlay (`overlay.rs`, `gamelog.rs`, `OverlayWindow.svelte`): opt-in, prozirni always-on-top prozor, globalni prečaci Alt+Shift+O / Alt+Shift+I, clipboard watcher samo dok je overlay vidljiv i samo za tekst koji počinje s `Item Class:`, praćenje `Client.txt` od zadnjih 512 KiB. Bez čitanja memorije igre i bez slanja inputa.
- Provjereno uživo nad pokrenutom igrom (PoE1): item kartica, map kartica i leveling panel iz stvarnog `Client.txt`. Progression panel u overlayu nije viđen uživo jer nema spremljenog para snapshotova; logika je ista kao u Progression tabu.
- Nije provjereno: macOS (overlay je tamo neproziran), PoE2 klijent uživo, ne-engleski klijenti, CI za ovu granu.

## Trade pretraga iz asistenta — 2026-11-16

- Grana `feat/trade-search-tool` (iz `feat/companion-overlay`), ništa nije pushano.
- PoB-ov Trader (`TradeQueryGenerator`) generira query bez trade API-ja; API treba samo za dohvat listinga. Novi bridge `trade_search_url` vrti postojeći `trade_search_start/step/result` do kraja i od queryja gradi link (`/trade/search/[realm/]<league>?q=` ili `/trade2/search/poe2/<league>?q=`) te sažetak: kategorija, težine, vagani modovi, `minWeight`. Weighting nije reimplementiran. Prvi put koristi isporučeni `Data/QueryMods.lua`; liga bez argumenta dolazi iz javnog `/api/trade/data/leagues` kao kod timeless pretrage.
- Alat `open_trade_search` (PoE1 87, PoE2 81): `read_only` + `open_world` pa ga panel tretira kao lookup koji korisnik odobrava; `slow`. Otvara link kroz `trade::open` (izdvojeno iz `trade_window_open`) i vraća `tradeUrl`, pa panel prikazuje i Open trade site gumb. Ne dohvaća listinge, ne koristi cookies/OAuth, ne šalje whisper i ne kupuje. Opis upućuje na Copy na listingu + overlay item check.
- Testovi: `crates/pob-engine/tests/trade_search.rs` (Ring 1, Life težina; kategorija `accessory.ring`, Life među vaganim modovima, kodiran URL, nepoznat stat odbijen, build nepromijenjen) prolazi na PoE1 i PoE2 podacima.
- 2026-11-17 uživo (`a16407e`, preview identitet, `tauri dev`, PoE1): `bun run check:tools` nad pokrenutim MCP serverom prolazi sve provjere. MCP `tools/call open_trade_search` (Ring 1, Standard, Life) vratio je `opened: true`, kategoriju `accessory.ring` i vagane Life modove; trade prozor otvoren. Prazan slot daje `minWeight` 0.
- Nije provjereno: odobravanje alata kroz chat panel, i počinje li tekst s trade siteovog Copy gumba s `Item Class:` (overlay watcher prihvaća samo takav tekst).
- Provjereno: `cargo test -p pob-redux tools::` (3 passed, brojevi 87/81), `bun run check` (0 grešaka, 0 upozorenja). `bun run check:tools` traži pokrenut app s MCP serverom pa nije pokrenut; ista CORE/ALLOWED/intent provjera izvedena je offline nad opisima iz `tools.rs` (14/14 intenta, novi "find a ring to buy on the trade site" → `open_trade_search`).
- Worktree nema `src-tauri/resources/pob*`; za `cargo test -p pob-redux` povezani su junctionima na glavni checkout (git ih ignorira).

## Uklanjanje Progression modula — 2026-11-17

- Korisnik je ocijenio Progression modul lošim i zatražio da se potpuno ukloni; fokus forka su AI asistent i overlay. Grana `feat/drop-progression` (iz `feat/trade-search-tool`), ništa nije pushano.
- Uklonjeno: `progression.lua` i `progression_capture`, `src-tauri/src/progression.rs` i njegove komande, alati `get_progression` / `capture_progression_snapshot` / `select_progression_snapshots` (PoE1 87 → 84), Progression tab i navigacija, `src/lib/progression/`, overlay Progression panel i postavka, Progression testovi i CI koraci, `PROGRESSION.md`, odlomak u promptu i stringovi.
- Overlay leveling sada uzima gemove samo iz otvorenog builda (prije iz Progression targeta ako postoji).
- Zadržano: preview identitet (`tauri.progression.conf.json`, `is_progression_preview`, `VITE_PROGRESSION_VALIDATION`) jer odvaja podatke od upstream aplikacije i gasi upstream OAuth/updater; ime ostaje do odluke o vlastitom identitetu. Import UI test premješten u `tests/ui` (`bun run test:ui`). Windows workflow preimenovan u "Windows preview build" i pokreće se za `feat/**`.
- Korisnikov `progression-poe1.json` u app local data nije brisan; aplikacija ga više ne čita. Povijesni unosi u ovom logu i `mistakes.md` ostaju kao zapis.
- Provjereno: `cargo test -p pob-redux --lib` (71 passed, 1 ignored), `cargo test -p pob-engine` na PoE1 podacima (svi engine testovi, uključujući item/map evaluate i trade search), `bun run check` (0/0), `bun test scripts` (19 pass), `bun run test:ui` (3 passed, Edge). Offline provjera popisa alata: svi intenti prolaze, nijedan alat nije nedostupan. `tauri dev` se ponovno izgradio s novim kodom i radi; živi `check:tools` nije ponovljen jer je MCP server na 7316 u međuvremenu ugašen u aplikaciji.

## Asistent i overlay — 2026-11-17

- Grana `feat/assistant-overlay` (iz `feat/drop-progression`), ništa nije pushano. Korisnik je odobrio svih šest prijedloga.
- **Trade Copy → overlay.** Trade stranica u ugrađenom pregledniku traži Cloudflare provjeru, pa stvarni format njenog Copy teksta nije viđen. Classifier sada prihvaća i tekst bez `Item Class:` ako počinje s poznatim `Rarity:` i ima `--------` separatore; karta se prepoznaje po `Map Tier:` / `Waystone Tier:`. Rust test pokriva oba oblika. Uživo s trade stranice i dalje nije provjereno.
- **`get_game_context`** (oba igre, PoE1 85 / PoE2 82 alata): zadnjih 5 kopiranih itema/mapa (samo u memoriji, samo dok je overlay vidljiv i panel uključen), svaki ocijenjen s `item_evaluate`/`map_evaluate` (bez `before`/`after` radi veličine) plus poe.ninja cijena, i stanje iz `Client.txt` bez putanje.
- **poe.ninja cijene** (`prices.rs`, PoE1): unique, currency, fragmenti, scarabi, esencije, fosili, oils, omeni, tattooi i div karte. Cijeli popis kategorije preuzima se i drži 30 min (`/poe1/api/economy/stash/current/{item|currency}/overview`), liga iz `/poe1/api/data/index-state` ili postavke `priceLeague`. Linkovi 5L/6L se preskaču. Ignorirani live test prošao 2026-11-17 (Kaom's Way Standard 10134c / 11,2 div; Divine Orb Covens 83,67c).
- **Overlay kartice:** cijena ispod naslova; kartica samo s cijenom za currency/karte koje ne idu ni u jedan slot; gumb **Ask the assistant** u interact modu stavlja tekst u polje za poruku (`overlay_ask` → `overlay:ask`; glavni prozor ili izdvojeni prozor asistenta), ništa se ne šalje samo.
- **Shopping lista:** `open_trade_search` ima `open: false`; chat ispod odgovora pokazuje gumb po slotu ("Trade: Ring 1"). Prompt upućuje asistenta da nakon `optimise_gear` napravi po pretragu za svaki slot.
- Registar alata je 59 997 bajtova uz tripwire od 60 000; opisi novih alata su skraćeni da stanu. Sljedeći alat traži skraćivanje ili svjesno podizanje limita.
- `cargo clippy` javlja greške u postojećem kodu (`acp.rs`, `agent/mod.rs`, `art.rs`, `gamelog.rs:133` "read amount is not handled", `ggg.rs`, `lib.rs`, `voice.rs`, `web.rs`); CI ne pokreće clippy. Nijedna nije u novim datotekama.
- **Hrvatski jezik** (`messages/hr.json`, "Hrvatski" u izboru jezika): 541 tekst, isti opseg kao ostali djelomični prijevodi (njemački pokriva 212) plus chat, asistent, overlay, postavke i paleta naredbi; ostatak pada na engleski. Nazivi iz igre (DPS, EHP, life, resist, unique, currency) ostaju engleski. Rečenice su rodno neutralne. Skripta je provjerila da svaki ključ postoji u `en.json` s istim `{placeholderima}`. Usput ispravljen zastarjeli engleski opis leveling panela (spominjao je target build).
