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

## Validacija — potvrđeno lokalno

- Bun 1.4.4 i dependency install iz lockfilea rade.
- `bun test scripts`: **25 pass, 0 fail**, uključuje 15 novih Progression unit testova i 10 postojećih testova; zadnji prolaz uključuje strožu validaciju backupova.
- `bun run check`: **0 errors, 0 warnings** nakon markup ispravka. Ponovljeni check nakon commita `61ebd64` također je završio s 0 errors / 0 warnings.
- `bun run build`: uspješan produkcijski frontend bundle; ostaje Vite upozorenje za veliki bundle. To nije potvrda native Windows builda.
- Playwright component-flow testovi: **3 pass** na lokalnom Edgeu; uvoz/greška, current/target i reload/delete, očuvanje oštećene povijesti. Ovo su IPC fixture testovi, ne PoB engine testovi.
- UI pregledan kroz renderirane screenshotove, bez horizontalnog overflowa na 1100 px; nema page-error događaja.

## Native CI — još otvoreno

- Prvi dovršeni Windows pokušaj [43715258887](https://github.com/minimalguru-dev/pob-redux/actions/runs/43715258887) prošao je frontend check/test/build, ali pao u LuaJIT buildu zbog `NoDefaultCurrentDirectoryInExePath` / `minilua`. Detalji i ispravak su u mistakes.md.
- [Windows run 43716010378](https://github.com/minimalguru-dev/pob-redux/actions/runs/43716010378) potvrdio je LuaJIT ispravak i browser testove. Pao je zatim na CRLF patchu; uzrok lokalno reproduciran i ispravljen pravilom `*.patch text eol=lf`. Idući run treba pratiti do kraja.
- Otvaranje PR-a također je pokrenulo [Linux check 43716041218](https://github.com/minimalguru-dev/pob-redux/actions/runs/43716041218). Pratiti oba.
- `crates/pob-engine/tests/progression.rs` uključuje Lua compile test i ignored real-engine test koji workflow eksplicitno pokreće sa synced PoE1 podacima. Provjerava parity, non-mutation, postojeći Compare state, invalid import i scenario jednakost direktnom PoB configu.
- Privatni upstream `pobredux/corpus` i `CORPUS_DEPLOY_KEY` nisu dostupni; puni golden suite nije verificiran.
- Lokalni Windows host nema Rust/MSVC/CMake preduvjete. Ne prijavljivati lokalno izgrađen native binary.

## Reprodukcija

Standardno: `bun install --frozen-lockfile`, `bun run check`, `bun test scripts`, `bun run build`.

Browser: `bun x playwright install chromium`, zatim `bun run test:progression:ui`. Lokalno je upotrijebljen `PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH=C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe`.

Native: slijediti CONTRIBUTING.md i Windows workflow. Nakon synca postaviti `POB_REDUX_POB_ROOT=src-tauri/resources/pob1`; pokrenuti `cargo test -p pob-engine --test progression -- --ignored --nocapture`. Za svaki Windows build proces ukloniti `NoDefaultCurrentDirectoryInExePath`, ne postaviti ga na prazan string.

## Sljedeći koraci pri preuzimanju

1. Provjeriti `git status`, zadnji commit i aktualne GitHub Actions runove na grani. Ne restartati analizu repozitorija niti tražiti već odobrene potvrde.
2. Dovršiti pokrenuti frontend check, Windows native test/build i Linux PR checks. Ako padnu, pročitati konkretan failed log i ažurirati mistakes.md prije ispravka.
3. Nakon uspješnog native testa preuzeti installer artifact i točno označiti što je provjereno (build/engine/GUI), a što nije.
4. Ažurirati ovaj log i PR opis stvarnim rezultatima. Pripremiti lokalni source bundle/patch i sažetak na hrvatskom s linkovima.
5. Ne mergeati PR niti objavljivati produkcijski release bez potrebe iz korisnikova opsega; unsigned validation artifact je predviđena isporuka.

## Dnevnik nastavka — 2026-11-15

- Na korisnikov zahtjev uvedeni mistakes.md i ovaj development log; commit `b84e2cc` mijenja samo dokumentaciju i nosi `[skip ci]` kako ne bi prekidao već pokrenutu kompilaciju nepromijenjenog koda.
- Korisnik je potvrdio nastavak. Rad se nastavlja unutar istog opsega, bez novih autorizacijskih pitanja.
- Završni pregled pooštrio je validaciju backupova (unique node IDs/slots, scalar tipovi, datum i loadout). Nakon promjene prolazi 25 unit + 3 browser testa.
- U Windows pipelineu razlikovana su dva potvrđena uzroka: minilua environment (riješen) i CRLF patch (lokalno dokazano, ispravak spreman za CI).
- Linux workflow dobiva eksplicitni real-engine Progression test prije pune workspace kompilacije, da runtime regresija bude vidljiva i na Linuxu.
- Commit `40df7a6` sadrži CRLF pravilo i backup validaciju; lokalni Svelte check 0/0 i frontend build ponovno prolaze.
- Linux run `43716753722` otkrio je cwd grešku novog test harnessa prije boota enginea. Test sada relativni PoB path razrješava iz workspacea; dodatno testira aktivni Ruby Flask i njegovo isključenje u boss scenariju. Runtime još nije potvrđen.
- MCP kod dodatno pregledan: `src-tauri/src/mcp.rs` bind na `127.0.0.1`, Bearer provjera. Nije pokrenut javni server niti promijenjena autentikacija.
