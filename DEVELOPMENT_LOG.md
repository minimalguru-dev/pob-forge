# Development log — PoB Forge Progression

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

## Rebrand u PoB Forge — 2026-11-19

- **Zahtjev:** korisnik želi projekt kao vlastiti proizvod: novo ime, suptilno drugačiji UI, QoL, bez upstream brendiranja. Odluke korisnika: ime **PoB Forge**; atribucija samo u `LICENSE` (MIT to zahtijeva; dodan i korisnikov copyright red, Juddov i PoB Community ostaju); vlastiti Cloudflare update/art server. Git povijest nije prepisivana.
- **Identitet:** productName/naslovi/poruke `PoB Forge`; identifier `dev.minimalguru.pobforge`; Cargo paket `pob-forge` (`pob_forge_lib`); env prefiks `POB_FORGE_*`; localStorage `pob-forge:`; MCP server ime `pobforge` (postojeće AI klijent konfiguracije treba ponovno dodati). Preview identity (`dev.minimalguru.pob-progression-preview`) ostaje.
- **Upstream servisi uklonjeni:** updater endpoint/pubkey, `art.pobredux.com`, Discord gumb, AUR paket i workflow, FUNDING, pobredux.com linkovi, privatni corpus repo (sada `vars.CORPUS_REPOSITORY`). GGG OAuth client ID više nije u kodu (vidi mistakes.md).
- **Build-time postavke** (`src-tauri/src/brand.rs`): `POB_FORGE_CDN_URL`, `POB_FORGE_GGG_CLIENT_ID`, `POB_FORGE_CONTACT`. Bez CDN-a tree-art/data download u release buildu prijavljuje jasnu grešku; dev buildovi koriste lokalni sync. `pob-sync` treba `POB_FORGE_CDN_URL` samo kad offline tree art nedostaje lokalno.
- **Updater:** `tauri.conf.json` ima prazan pubkey i GitHub endpoint. `release.yml` traži varijablu `TAURI_UPDATER_PUBKEY`, upisuje je i CDN endpoint prije builda te postavlja `VITE_UPDATER=true`; buildovi bez toga ne provjeravaju update.
- **CDN:** `infra/cdn` Worker nad R2: hashirani tree/data, item art s pull-through kopiranjem s `ORIGIN`, `update/latest.json` proxy na zadnji GitHub release. Nije deployan; nema Cloudflare pristupa iz ove sesije.
- **Migracija podataka:** `legacy.rs` prije kreiranja prozora kopira (ne premješta) config/data/local/cache mape starog identifiera (WebView profil bez cache mapa); `storage-migration.ts` preimenuje stare localStorage ključeve; keyring/DPAPI ključevi čitaju se sa starog namespacea i prepisuju.
- **UI/QoL:** tamna tema prebačena na topli charcoal (hue 58) s ember brand/focus bojom, light tema dobiva isti akcent; novi logo (čekić + žar) i sve ikone generirane iz `src-tauri/icons/source.svg`; Progression tab ima vlastitu ikonu (zastava); glavni prozor pamti veličinu, poziciju i maksimizirano stanje (`window_state.rs`, provjera da je naslovna traka na nekom monitoru).
- **Validacija lokalno (Windows, necommitano stanje):** `cargo check --locked --workspace --all-targets` prolazi; `cargo test -p pob-forge --lib` 90 passed, 1 ignored (postojeći voice); `bun test scripts` 27 pass; `bun run check` 0/0; `bun run build` prolazi; Playwright 6 pass (Edge). Vizualno provjereni naslovna traka i Progression u tamnoj i svijetloj temi kroz privremeni browser harness (obrisan).
- **Nije provjereno:** native desktop pokretanje novog builda, stvarna migracija s korisnikovim starim podacima, Worker na Cloudflareu, release workflow, CI na GitHubu. Repozitorij na GitHubu još se zove `pob-redux`; linkovi u kodu pretpostavljaju `minimalguru-dev/pob-forge` (GitHub preusmjerava nakon preimenovanja).

## CDN deploy — 2026-11-19

- Korisnik je uključio R2 i odobrio deploy. Bucket `pob-forge` (EEUR) i Worker `pob-forge-cdn` na `https://pob-forge-cdn.minimalguru.workers.dev`; verzija `d02d9715-463e-4166-a460-fed932040373`. `ORIGIN` je postavljen samo deploy naredbom (`--var`), u repozitoriju ostaje prazan.
- **Provjereno:** tree art (PoE1 i PoE2) povučen s origina, drugi zahtjev iz bucketa, SHA-256 jednak hashu iz `art.json`; `maps/poe1/latest.json` 200; item art `.webp?v=<tag>` 200; nepostojeći ključ 404; `update/latest.json` 404 jer repozitorij `minimalguru-dev/pob-forge` još ne postoji i nema releasea. Prvi zahtjevi odmah nakon deploya vratili su 404/500 u formatu koji nije Workerov (vjerojatno propagacija); ponovljeno ispravno.
- GitHub varijabla `POB_FORGE_CDN_URL` postavljena na `minimalguru-dev/pob-redux`.
- **Nije napravljeno:** `bun run downloads upload` (treba R2 S3 ključeve); bucket se puni samo na zahtjev. Mapa item arta sadrži polje `images` sa starom adresom; aplikacija ga ne koristi.
- **Custom domain (isti dan):** korisnikova zona `netrum.eu` je na računu. Worker je deployan na `cdn.netrum.eu` (verzija `6893af02-f1ab-4b8c-8fa4-1942e05f54f4`, `ORIGIN` opet samo kroz `--var`); workers.dev adresa je isključena. Provjereno: tree art 200 s ispravnim SHA-256, `maps/poe2/latest.json` 200, nepostojeći ključ 404, workers.dev 404. GitHub varijabla `POB_FORGE_CDN_URL` sada je `https://cdn.netrum.eu`.

## Price check u overlayu — 2026-11-19

- **Zahtjev:** price check kao Awakened PoE Trade. Odluke korisnika: puni opseg (poe.ninja + trade za rare), anonimni trade API, obje igre, prečac **Ctrl+D koji šalje Ctrl+C igri**. Pravilo u AGENTS.md izričito dopušta samo taj jedan Ctrl+C.
- **Rust:** `price.rs` — `price_ninja` (poe.ninja `poe1|poe2/api/economy/{exchange|stash}`, cache 10 min, svi tečajevi iz `core.rates`), `price_trade` (anonimni POST search + GET fetch 10 najjeftinijih, bez kolačića, UA s kontaktom, `ggg::trade_policy` i `wait_from` za GGG rate limit). `game_window.rs` — fokus prozora igre (`POEWindowClass`, naslov kao fallback), `SetForegroundWindow`, `SendInput` Ctrl+C; samo Windows. `overlay.rs` — Ctrl+D se registrira samo dok igra ima fokus (watcher 200 ms), čeka do 1 s na promjenu clipboarda (isti item drugi put = isti tekst), potiskuje dupli item card, otvara overlay u interact modu; `overlay_price_done` vraća fokus igri. Postavke `priceCheck`/`priceKey`.
- **Bridge:** `buy_similar_info/url` primaju `{ raw }` bez otvorenog builda; `buy_similar_url` vraća i JSON upita (privremeni hook na `tradeHelpers.B64GzipEncode` upvalue iz PoB-ovog `buildURL`). Info dobiva `rarity`, `itemLevel`, `corrupted`.
- **Frontend:** `src/lib/overlay/price.ts` (parsiranje kopiranog teksta, izbor ninja tablica, matching gem level/quality/corrupted i 5/6L, formatiranje c/div/ex) i `PriceCard.svelte` (liga po igri u localStorageu, ninja cijena, modovi s min = 90 %, pretraga, listinzi, otvaranje u trade prozoru).
- **Validacija lokalno:** `bun test scripts` 32 pass (novi price testovi); `bun run check` 0/0; Rust `pob-forge --lib` testovi uključujući nove `price`/`game_window`; `pobctl call buy_similar_info|buy_similar_url` s `{raw}` na stvarnom PoB-u PoE1 i PoE2 (bez builda) daju modove i upit; taj PoE1 upit, omotan kao u `price_trade`, GGG trade API prihvatio je i vratio rezultate (jedna anonimna pretraga). poe.ninja endpointi provjereni curlom za obje igre. PriceCard vizualno provjeren s IPC fixtureima (headless Edge).
- **Nije provjereno:** Ctrl+D u pokrenutoj igri (SendInput, ime klase prozora, clipboard timing, povrat fokusa), PoE2 uniques na poe.ninja matching uživo, ponašanje pri GGG limitu. Ne-engleski klijenti nisu podržani.
- **Prvi test u igri (korisnik, isti dan):** Ctrl+D je okinuo dok je igra imala fokus (detekcija prozora radi), ali clipboard se nije promijenio: igra nije primila simulirani Ctrl+C. Overlay je lokalno provjeren simuliranim clipboardom: watcher bilježi kopiju, kartica s PoB izračunom i leveling panel iscrtavaju se ispravno. Overlay je od 21:22 bio isključen u postavkama, pa ručni Ctrl+C u igri nije mogao stići.
- **Ispravak (neprovjeren u igri):** `SendInput` sada šalje i hardverski scan kod (`MapVirtualKeyW`) uz virtualnu tipku, a pritisak traje ~30 ms. `register_keys` više ne drži vlastiti mutex dok plugin registrira prečace na glavnoj niti (moguć deadlock s handlerom na toj niti). Dodani logovi: preuzimanje prečaca, poslan Ctrl+C, clipboard bez promjene. CI za lint ispravak: [44607126368](https://github.com/minimalguru-dev/pob-forge/actions/runs/44607126368) success.
- **Pozicija kartica (zahtjev korisnika):** price i Ctrl+C item/map kartica iz igre premještaju overlay tik lijevo od panela inventara (širina panela = 986/1600 visine prozora igre, kao Awakened), okomito na sredinu; overlay se vraća na svoje mjesto kad price kartica zatvori ili item kartica istekne (`overlay_card_closed`). Dok je price kartica otvorena, leveling i progression paneli su skriveni. Kopija s trade sitea u browseru ne pomiče overlay. Testirano samo unit testom pozicije; u igri neprovjereno.
