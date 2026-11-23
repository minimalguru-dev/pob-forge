# Potvrđene greške i kako ih ne ponoviti

Ovaj zapis vodi razvojni agent. Dodavati samo opažene greške ili dokazive nedostatke, uz ispravak i status provjere. Ne pretvarati pretpostavke u potvrđene uzroke. Datumi/vremena odnose se na Europe/Zagreb.

## 2026-11-23 — Windows preview workflow nije dobio adresu CDN-a

- **Simptom:** "Windows preview build" (`progression-windows.yml`) pada na svakom pushu `feat/**` grane od 2026-11-21 (npr. [run 44888797245](https://github.com/minimalguru-dev/pob-forge/actions/runs/44888797245), `bd125a8`), u koraku "Sync pinned PoB engines".
- **Dokaz:** log: `Error: tree art is missing locally and POB_FORGE_CDN_URL is not set, so it cannot be downloaded`.
- **Uzrok:** rebrand je maknuo `art.pobredux.com` i uveo build-time `POB_FORGE_CDN_URL`; `release.yml` i `check.yml` čitaju `vars.POB_FORGE_CDN_URL`, ali Windows preview workflow nije ažuriran. Neuspjesi nisu primijećeni jer se ishod tog workflowa nije provjeravao nakon pusha.
- **Ispravak:** `env: POB_FORGE_CDN_URL: ${{ vars.POB_FORGE_CDN_URL }}` na razini workflowa.
- **Prevencija:** kad se uvodi nova build-time varijabla, pretražiti sve `.github/workflows/*.yml` (ne samo release); nakon pusha provjeriti ishod svakog pokrenutog workflowa, ne samo onog koji se mijenjao.
- **Validacija:** [run 45253373627](https://github.com/minimalguru-dev/pob-forge/actions/runs/45253373627) na `e8aea01` uspješan: sync oba enginea, engine i app testovi, browser testovi, nepotpisani installer i lockfile provjera.

## 2026-11-15 — Lua string.sub reže bajtove, ne Unicode znakove

- **Nalaz u završnom pregledu:** `label:sub(1, 160)` skraćuje valjan naziv od 160 znakova s dijakriticima i može presjeći UTF-8 znak. HTML maxlength broji znakove, ne iste bajtove.
- **Ispravak:** sačuvati cijeli naziv i odbiti tek backend input veći od 640 bajtova; UI zadržava svoj maxlength 160.
- **Prevencija:** ne rezati korisnički Unicode tekst Lua byte substringom.
- **Validacija:** real-engine test uspoređuje točan naziv `A` + 159 slova `č`; uspješno prošao na `90cd6bb` u Linux runu `43718155297` i Windows runu `43718149574`.

## 2026-11-15 — Cargo integration test nema workspace root kao cwd

- **Simptom:** [Linux run 43716753722](https://github.com/minimalguru-dev/pob-redux/actions/runs/43716753722) uspješno je syncao podatke i kompajlirao engine test, ali prijavio da `src-tauri/resources/pob1` nema `Launch.lua`.
- **Uzrok:** Cargo test izvršava iz crate direktorija; relativni env path zamišljen kao workspace-relative nije razriješen prije `Engine::boot`.
- **Ispravak:** test zadržava apsolutni path ako je zadan, a relativni razrješava prema `CARGO_MANIFEST_DIR/../..`. Greške engine callova sada uključuju ime metode.
- **Prevencija:** testovi ne smiju pretpostavljati shell cwd. Put do testnih resursa graditi iz manifest direktorija ili koristiti apsolutni path.
- **Validacija:** real-engine korak Linux runa `43717329626` uspješan, uključujući boot i sva parity/non-mutation očekivanja. Izvorni pad bio je greška testnog puta, ne calculation enginea.

## 2026-11-15 — PowerShell ne proširuje wildcard putanju za rg kao POSIX shell

- **Simptom:** `rg ... src-tauri/src/mcp*` i sličan obrazac daje OS error 123.
- **Ispravak:** koristiti poznatu konkretnu datoteku ili `rg -g 'mcp*' ... src-tauri/src`.
- **Prevencija:** glob filtrirati kroz `rg -g`; ne ponavljati wildcard putanje kao argument datoteke u PowerShellu.
- **Validacija:** pregled `src-tauri/src/mcp.rs` uspješan; potvrđeni loopback bind i Bearer provjera.

## 2026-11-15 — Backup validacija mora provjeriti i jedinstvenost ključeva

- **Nalaz u završnom pregledu:** početna provjera formata dopuštala je duplikate passive node ID-ova. Takav backup može proizvesti duplicirane ključeve u Svelte listi i prekinuti render. Samo provjera da su vrijednosti brojevi nije dovoljna.
- **Ispravak:** odbaciti duplikate node ID-ova i equipment slotova; dodatno provjeriti scalar config/stats, loadout polja i datum.
- **Prevencija:** za svaku keyed listu koja dolazi iz uvoza validirati jedinstvenost ključa prije spremanja i rendera.
- **Validacija:** test za šest malformiranih snapshotova prolazi; ukupno 25 unit testova i 3 browser testa prolaze nakon ispravka.

## 2026-11-15 — Upstream patch datoteke također trebaju LF na Windowsu

- **Simptom:** [Windows run 43716010378](https://github.com/minimalguru-dev/pob-redux/actions/runs/43716010378) uspješno je izgradio LuaJIT i pob-sync, zatim pao na PoE2 `CalcOffence.lua` patchu.
- **Uzrok:** pob-sync već normalizira odredišni Lua tekst na LF, ali checkout samog `.patch` filea s `core.autocrlf=true` pretvara patch u CRLF. `.gitattributes` je ranije štitio samo `.sh` i `trees/**`.
- **Dokaz:** lokalni `git apply -p2 --check` na sirovom `CalcOffence.lua` iz točno pinanog PoE2 commita: CRLF patch exit 1; isti patch nakon isključivo CRLF→LF normalizacije exit 0.
- **Ispravak:** dodati `*.patch text eol=lf` u `.gitattributes`. Ne mijenjati formule, sadržaj patcha ili globalnu korisnikovu Git konfiguraciju.
- **Prevencija:** patch datoteke tretirati kao byte-sensitive build ulaz; provjeriti `git ls-files --eol patches` kad se identičan pin različito ponaša na Linuxu i Windowsu.
- **Validacija:** lokalni diskriminacijski test potvrđuje uzrok; Windows run `43717323768` prošao je sync oba PoB enginea.

## 2026-11-15 — LuaJIT: prazna varijabla nije uklonjena varijabla

- **Simptom:** Windows CI nije mogao pokrenuti `minilua` koji je upravo izgradio. `mlua-sys` build završio je greškom 101.
- **Dokaz:** [neuspjeli run 43715258887](https://github.com/minimalguru-dev/pob-redux/actions/runs/43715258887), korak `Sync pinned PoB engines`; stderr: `'minilua' is not recognized as an internal or external command`.
- **Uzrok:** `NoDefaultCurrentDirectoryInExePath` bila je upisana kao prazna vrijednost u `GITHUB_ENV`. Prisutnost varijable i dalje blokira traženje izvršnog programa u trenutnom direktoriju. Uklanjanje u jednom koraku također ne uklanja varijablu iz budućih shell procesa.
- **Ispravak:** ukloniti varijablu naredbom `Remove-Item Env:NoDefaultCurrentDirectoryInExePath -ErrorAction SilentlyContinue` u svakom procesu koji pokreće Cargo/Tauri. Ne dodavati praznu vrijednost u `GITHUB_ENV`.
- **Prevencija:** pročitati Windows napomenu u `CONTRIBUTING.md`; kod ovog simptoma prvo provjeriti naslijeđeno okruženje, a ne mijenjati LuaJIT ili PoB izvorni kod.
- **Validacija:** ispravak u commitu `61ebd64` potvrđen runom `43716010378`: LuaJIT i pob-sync izgrađeni. Sljedeća greška bila je zasebni CRLF patch problem.
- **Ponovljeno 2026-11-19 lokalno:** Git Bash na `D:\dev` ima `NoDefaultCurrentDirectoryInExePath=1`; `cargo check` je pao s istim `minilua` stderrom. PowerShell s `Remove-Item Env:NoDefaultCurrentDirectoryInExePath` u istom procesu riješio je problem bez promjene koda. Lokalne Cargo naredbe pokretati tako, ne iz Git Basha.

## 2026-11-15 — Svelte upozorenje ruši obvezni check

- **Simptom:** `bun run check` završio je kodom 1 uz 0 errors / 1 warning.
- **Uzrok:** samozatvarajući `<textarea />`; HTML ga ne tretira kao void element. Projekt koristi `--fail-on-warnings`.
- **Ispravak:** eksplicitni `</textarea>`; commit `fcc2efa`.
- **Prevencija:** provjeravati i upozorenja, ne samo broj TypeScript grešaka.
- **Validacija:** ponovljeni Svelte check: 0 errors / 0 warnings; produkcijski frontend build prošao.

## 2026-11-15 — Upotreba nepostojećih theme tokena

- **Nalaz:** novi view koristio je `--border`, `--text` i `--text-dim`, kojih nema u upstream dizajnu.
- **Ispravak:** koristiti `--line-1`, `--fg-1`, `--fg-3` iz `src/app.css`; commit `fcc2efa`.
- **Prevencija:** prije oblikovanja novog viewa provjeriti stvarne tokene i susjedni view. Uspješan build ne potvrđuje CSS izgled.
- **Validacija:** pregled snimke stvarno renderiranog Progression viewa; provjeren overflow na širini 1100 px.

## 2026-11-15 — Lua: `and/or` nije ternarni operator za false

- **Nalaz u pregledu prije prvog commita:** izraz `condition and false or 0` vraća 0, koji je u Lui truthy. Kill-condition checkbox mogao bi ostati efektivno uključen.
- **Ispravak:** eksplicitni `if/else` za booleane i numeričke vrijednosti. Isto vrijedi za kopiranje boolean statova u JSON.
- **Prevencija:** ne koristiti Lua `and/or` ternarni obrazac kada je srednja vrijednost `false` ili `nil`.
- **Validacija:** integration test eksplicitno očekuje `false` za `conditionKilledRecently` i uspoređuje boss rezultate s direktnim PoB izračunom; prošao u Linux runovima `43717329626` i `43718155297`.

## 2026-11-15 — Doseg Lua lokalnih varijabli između modula

- **Nalaz u pregledu prije prvog commita:** novi modul referencirao je `GAME`, ali je ta varijabla lokalna u `bridge.lua`.
- **Ispravak:** game provjeru čitati preko proslijeđenog `methods.version().game`.
- **Prevencija:** kod izdvajanja modula eksplicitno proslijediti ovisnosti; ne pretpostavljati da je bridge-local stanje globalno.
- **Validacija:** stvarni engine capture uspješno izvršen u Linux runovima `43717329626` i `43718155297`.

## 2026-11-15 — Browser test i dostupno ime select kontrole

- **Simptom:** test je istekao tražeći select točno nazvan `Use as`.
- **Uzrok:** implicitni label sadržavao je i tekst opcija.
- **Ispravak:** dodani eksplicitni `aria-label` atributi za role i boss select; zadržani vidljivi labeli.
- **Prevencija:** testirati preko dostupnih imena kontrola; ne skrivati problem zamjenom testa nečitljivim CSS selektorom.
- **Validacija:** sva 3 Playwright testa prolaze, uključujući promjenu role i dva uspješna capturea.

## 2026-11-15 — Dostupnost browsera ne pretpostavljati

- **Simptom:** povezani Playwright alat nije našao Chrome na svom zadanom putu.
- **Ispravak:** lokalni testovi koriste instalirani Edge uz eksplicitni executable path; reproducibilni CI instalira Playwright Chromium.
- **Prevencija:** provjeriti browser/runtime prije testiranja. Nedostajući browser nije greška aplikacije.
- **Validacija:** lokalni headless Edge testovi prošli bez browser grešaka.

## 2026-11-15 — Preview identity ne izolira spremljene prijave

- **Nalaz pri korisnikovoj sigurnosnoj provjeri:** zasebni Tauri identifier ne mijenja hardkodirani `SERVICE = dev.pobredux.desktop` u `src-tauri/src/ai.rs`. PoE secret `ggg-oauth` zato koristi isti Credential Manager namespace kao upstream. Windows DPAPI fallback file prati zasebni app config direktorij, ali primarni Credential Manager ne.
- **Dodatni kontekst:** `ggg.rs` zadržava upstream OAuth client ID `pobredux` i scopeove `account:profile account:characters account:trade`. Nije potvrđeno zasebno odobrenje za distribuciju forka pod tim klijentom. Codex integration pokreće korisnikov lokalni CLI bez zasebnog CODEX_HOME i može koristiti njegov postojeći auth profil.
- **Propust u ranijem pregledu:** potvrđeni build/testovi i odvojeni app identity nisu puni auth security audit. Nije opažena krađa tokena niti kompromitacija.
- **Prevencija:** kod izolacije previewa provjeriti i credential namespace, OAuth registraciju i profil vanjskog CLI-ja; ne zaključivati izolaciju iz Tauri identifiera.
- **Status:** nalaz dokumentiran; binary nije mijenjan. Preporuka za sada: Progression testirati code/XML importom bez autorizacije računa; prije autorizacije glavnih računa razriješiti navedene granice. Nisu čitani korisnikovi tokeni, lozinke ni auth cache.
- **Nastavak 2026-11-19 (rebrand u PoB Forge):** upstream OAuth client ID uklonjen iz koda. Prijava na pathofexile.com radi samo ako build dobije vlastiti `POB_FORGE_GGG_CLIENT_ID`; bez njega backend odbija sign-in/token. Keyring SERVICE i DPAPI entropija sada su `dev.minimalguru.pobforge`; stari zapisi čitaju se jednom i prepisuju (`ai.rs`, `legacy.rs`). Rust unit testovi prolaze; stvarna prijava nije testirana jer client ne postoji.

## 2026-11-15 — Paste action silently discarded clipboard failures

- **Evidence:** ImportView used `readText().catch(() => "")`; both a permission/clipboard-lock error and an empty clipboard ended with no message. The exact clipboard condition on the user's first click was not captured.
- **Fix:** bounded retries for transient read failures, inline errors for empty/denied clipboard and invalid imports, busy feedback and a Ctrl+V fallback. URL, raw code and XML dispatch are exercised by actual button-click component tests with mocked IPC.
- **Validation:** 28 Bun tests, 6 browser tests, Svelte 0 errors / 0 warnings and frontend build pass. Native UI remains unverified: desktop tooling failed to initialise and automatic approval rejected remote-debugging launch.

## 2026-11-15 — Cache cleanup can remove another download's temporary file

- **Evidence:** tree cleanup deleted every unlisted filename, including temporary writes from concurrent PoE1/PoE2 downloads. The original Windows log only said `could not write`, so this code defect is not conclusively the cause of that particular failure. C: had about 500 MB free; disk exhaustion is a hypothesis, not a confirmed cause.
- **Fix:** cleanup only retires content-hash art filenames; temporary writes include process ID and retain the original I/O error. A regression checks that in-flight and unrelated files survive cleanup selection, and atomic writes replace files without swallowing I/O failures.
- **Local recovery:** restored 81 missing art files (12,485,354 bytes) from the existing official art endpoint, checking size and SHA-256 before installation in the app cache. All 990 manifest entries across both games, including bundled offline copies, now match their hashes.
- **Pending:** native CI validation of the changes and actual in-app rendering. No user files or system caches were removed to free disk space.

## 2026-11-15 — Audit scripts must filter null advisory IDs

- **Evidence:** PowerShell wrapped a missing `vulns` property as an array containing null, initially counting all 657 packages as findings.
- **Fix:** filter empty IDs before counting; inspected the corrected advisory records and rescanned after upgrades. Only non-empty returned advisory IDs are reported.
- **Prevention:** distinguish an empty result from an array containing null; inspect representative results before summarising a scanner.

## GitHub CLI repository selection in a fork

- Symptom/evidence: unqualified `gh run list` displayed upstream runs; workflow dispatch against upstream returned 404/403. No upstream workflow was started.
- Cause: gh selected the upstream repository in this fork checkout.
- Fix/prevention: always pass `-R minimalguru-dev/pob-redux` for CI, artifact and PR operations. The feature-branch runs were already running in the correct fork.
- Validation: explicit repository lookup found Windows 43724471695 and Linux 43724475298 for b9217741.

## Codex credential-store argument through the login terminal

- Source review found that the Windows terminal wrapper replaces double quotes. A double-quoted TOML value in the new Codex override would be rewritten during login.
- The override now uses TOML single quotes, preserved by the existing PowerShell quoting helper. The direct app-server launch receives the same value.
- Verified a PowerShell parse round trip without invoking Codex or authenticating; the native launch regression also asserts the terminal transformation preserves the quoted argument.

## Incomplete shared writer call-site update

- Linux run 43724475298 failed with E0308 in src-tauri/src/threads.rs:29: the shared art writer now returns Result but this caller still expected bool.
- The initial call-site search was too narrow. Searched all Rust files from the repository root and updated the assistant thread writer to retain the OS error as well.
- Prevention: search all workspace references when changing a shared function signature, and require native workspace compilation before declaring the change validated. Frontend checks cannot detect Rust call-site errors.

## Native HTTP test must initialize the TLS provider

- Runs 43725402086 (Windows) and 43725407072 (Linux) failed only in the new OAuth redirect test: reqwest rustls-no-provider panicked because the test bypassed application startup.
- Production run() already calls pob_engine::install_tls_provider(). The test now calls that same initialization before constructing the real HTTP client. Redirect protection remains unchanged; no TLS checks are disabled.
- Prevention: when testing a production client outside the application entry point, reproduce its required initialization. Revalidate on both platforms rather than marking the test ignored.

## Line-range cut removed the next function's attribute

- Removing `update_method` from `src-tauri/src/lib.rs` with a scripted cut ("from this line up to the line of `fn app_paths`") also removed the `#[tauri::command]` above `app_paths`. `cargo check` failed: `cannot find macro __tauri_command_name_app_paths` at the `generate_handler!` list.
- Cause: the end marker was the function line, not the attribute above it, so the attribute fell inside the removed range.
- Fix: restored the attribute; `cargo check --workspace --all-targets` and the app tests passed.
- Prevention: end a scripted cut on the next item's first attribute or doc comment, or diff the result before compiling. The compiler catches a lost `#[tauri::command]` only when the command is registered; a lost `#[test]` or `#[cfg]` would compile silently.

## Git Bash's link shadowed MSVC's linker in check.yml

- The first Windows run of `check.yml` (bash as the default shell) failed in every build script: `linking with link.exe failed`, `/usr/bin/link: extra operand` ([run 44562524049](https://github.com/minimalguru-dev/pob-redux/actions/runs/44562524049)).
- Cause: Git Bash puts `/usr/bin` (coreutils `link`) ahead of the PATH entries `ilammy/msvc-dev-cmd` adds. `progression-windows.yml` uses pwsh and never hit it. The workflow was written without a run on GitHub.
- First fix was partial: `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` covered rustc, but LuaJIT's `msvcbuild.bat` calls `link` by name and failed the same way ([run 44569639450](https://github.com/minimalguru-dev/pob-redux/actions/runs/44569639450)).
- Fix: the `BASH_ENV` file prepends MSVC's `bin/Hostx64/x64` to PATH for every bash step, and a step asserts `command -v link` is MSVC's. [Run 44570971876](https://github.com/minimalguru-dev/pob-redux/actions/runs/44570971876) passed on `4feb5bc`.
- Prevention: on Windows runners, fix the tool lookup (PATH), not one consumer of it; check every native build that shells out (LuaJIT, cmake) uses the same tool. Run a new workflow with `workflow_dispatch` before claiming it works.

## 2026-11-19 — Masovni rename prepravio povijesne zapise

- **Simptom:** skripta za rename `PoB Redux` → `PoB Forge` zamijenila je i tekst u `DEVELOPMENT_LOG.md` i `mistakes.md`: upstream link pokazivao je na vlastiti fork, a zapis o starom keyring SERVICE-u naveo je novu vrijednost kao da je bila stara.
- **Dokaz:** `git diff DEVELOPMENT_LOG.md mistakes.md` prije commita; npr. `[Upstream](https://github.com/minimalguru-dev/pob-forge)` i `SERVICE = dev.minimalguru.pobforge` u zapisu od 2026-11-15.
- **Uzrok:** rename je primijenjen na sve praćene tekstualne datoteke, uključujući dokumente koji bilježe što je bilo istina u prošlosti.
- **Ispravak:** obje datoteke vraćene na HEAD prije commita; promijenjen samo naslov loga i dodani novi unosi. Ništa netočno nije commitano.
- **Prevencija:** kod masovnih zamjena isključiti povijesne/handoff zapise i pregledati njihov diff zasebno; povijesne CI linkove ostaviti na staroj adresi (GitHub preusmjerava nakon preimenovanja repozitorija).
- **Validacija:** `git diff` obiju datoteka sadrži samo naslov i nove unose.
- **Ponovljeno 2026-11-20 (merge `feat/assistant-overlay`):** ista skripta pokrenuta nakon mergea prepravila je `legacy.rs`, `storage-migration.ts` i njegov test (stari identifier i `pob-redux:` prefiks koje migracija namjerno traži) te `mistakes.md`/`DEVELOPMENT_LOG.md`. Uhvaćeno u izlazu skripte prije commita; datoteke vraćene (`git checkout HEAD`, `git checkout -m` i ponovno rješavanje). Skripta sada mora isključiti i migracijske datoteke.

## 2026-11-19 — Windows-only kod ostavio neiskorištenu funkciju na Linuxu

- **Simptom:** CI run [44605389235](https://github.com/minimalguru-dev/pob-forge/actions/runs/44605389235) (Linux) pao je u `Compile the workspace`: `function is_game is never used`, a lokalni Windows `cargo check --locked --workspace --all-targets` prolazio je.
- **Uzrok:** `game_window::is_game` zove samo `#[cfg(windows)]` implementacija; na Linuxu je ostala neiskorištena, a lintovi tretiraju upozorenja kao greške.
- **Ispravak:** `#[cfg_attr(not(windows), allow(dead_code))]` na `is_game`; testovi je i dalje pokrivaju na svim platformama.
- **Prevencija:** kod koda iza `#[cfg(windows)]` provjeriti i ne-Windows granu; lokalni Windows check ne zamjenjuje Linux CI.
- **Validacija:** ponovljeni Linux CI nakon ispravka.

## 2026-11-19 — Price check odbijao ispravan trade search id

- **Simptom:** u igri Ctrl+D javlja "pathofexile.com sent no search id".
- **Dokaz:** stvarni odgovor `/api/trade/search` ima id oblika `H4sIAAAA…PB_3WP…` (base64url gzipanog upita), a `price_trade` je dopuštao samo `[A-Za-z0-9]`.
- **Uzrok:** pretpostavka o formatu ID-a bez provjere na stvarnom odgovoru; ranija ručna provjera API-ja vidjela je id, ali nije uspoređena s filterom.
- **Ispravak:** `valid_search_id` dopušta i `-` i `_` (do 4096 znakova) i dalje odbija sve što bi promijenilo URL; unit test s base64url id-jem.
- **Prevencija:** validaciju vanjskih ID-jeva pisati prema stvarnom uzorku odgovora i taj uzorak staviti u test.
- **Validacija:** unit test; ručni anonimni search + fetch s istim id-jem vratio listinge. U igri neprovjereno do novog installera.

## 2026-11-20 — Testiranje trade API-ja curlom zaključalo IP na 30 minuta

- **Simptom:** `Rate limit exceeded; Please wait 1800 seconds` na anonimnom trade searchu.
- **Dokaz:** u kratkom razmaku poslano je desetak pretraga curlom (Foulborn filter, PoE2 grupe filtera, crafted/fractured), s pauzom od 2 s, bez čitanja `X-Rate-Limit-*` zaglavlja.
- **Uzrok:** ručni testovi zaobišli su ograničenje koje aplikacija poštuje (`price.rs` čita zaglavlja); GGG kažnjava prekoračenje dugom blokadom za cijeli IP, pa i korisnikov price check.
- **Ispravak:** testovi zaustavljeni; korisnik obaviješten da trade rezultati ne rade do isteka blokade.
- **Prevencija:** najviše 3–4 ručne pretrage u minuti, pročitati `X-Rate-Limit-Ip` i `-State` prije sljedeće; za pitanja tipa "postoji li filter" koristiti `/data/filters` i `/data/stats` (nisu search).
- **Posljedica (isti dan):** korisnik je u tom razmaku javio da price check ne nalazi Forbidden Tome. PoB ga ne čita pa kartica ide na običnu pretragu po imenu, a ona je u tom trenutku dobivala blokadu; nakon isteka ista pretraga (`type: "Forbidden Tome"`) vraća rezultate. Točna poruka koju je korisnik vidio nije potvrđena.
- **Pravilo iz zaglavlja:** `X-Rate-Limit-Ip: 5:10:60,15:60:300,30:300:1800,600:21600:3600` – 5 pretraga u 10 s, 15 u minuti, 30 u 5 min (inače 30 min blokade), 600 u 6 h. Ručni testovi i korisnikovi price checkovi dijele isti IP i isti brojač.
- **Validacija:** blokada istekla ~20:20; dvije pretrage nakon toga prošle su (stanje 2:300).

## 2026-11-20 — Bash heredoc pojeo backslasheve u regexima

- **Simptom:** `/\(mutated\)$/` završio kao `/(mutated)$/`, `\r` kao stvarni CR, `\b` kao backspace; test parsiranja qualityja pao.
- **Dokaz:** isti TS skript zapisan kroz Bash heredoc dao je datoteku bez backslasheva; zapisan Write alatom radi ispravno.
- **Uzrok:** Bash alat na Windowsu ne čuva `\\` u heredocu ni s navodnicima oko oznake.
- **Ispravak:** pogođene linije popravljene; skripte s backslashevima pišu se Write alatom pa pokreću Bunom.
- **Prevencija:** nikad heredoc za kod s regexima ili escape sekvencama; nakon zapisa grep za `\d`/`\s` u izmijenjenim linijama.
- **Validacija:** `bun test scripts` 34 pass.
- **Ponovilo se 2026-11-21:** `bun -e '…'` u Bash alatu s template literalom: `/:\\s*$/` završio je u Sidebar.svelte kao `/:s*$/`. Isti uzrok (escape kroz Bash), isto pravilo: skripte s backslashevima samo kao datoteka (Write) pa `bun datoteka.ts`, ili Edit alat. Nađeno grepom za `replace(/` nakon izmjene; ispravljeno Edit alatom, `bun run check` 0/0.

## 2026-11-20 — Ctrl+D ponekad otvorio i compare karticu

- **Simptom:** korisnik javlja da Ctrl+D ponekad pokrene compare.
- **Dokaz:** clipboard watcher reagira na svaki Ctrl+C koji vidi kroz `GetAsyncKeyState`, a price check šalje vlastiti C dok korisnik drži Ctrl. `price_busy` se gasio prije nego što je `suppress` hash bio spremljen, i odmah nakon slanja, bez rezerve za kasno javljeno stanje tipke. Točan put u igri nije reproduciran.
- **Uzrok (vjerojatan):** utrka između gašenja `price_busy` i watcherovog uzorka tipki, pa je poslani C izgledao kao korisnikov.
- **Ispravak:** `PriceBusy` guard gasi zastavicu tek nakon spremanja hasha i na svakom izlazu; još 400 ms (`PRICE_QUIET`) watcher ignorira Ctrl+C.
- **Prevencija:** kod simuliranog unosa, stanje "naš unos u tijeku" držati dok svi tragovi tog unosa ne prođu, ne samo dok traje slanje.
- **Validacija:** Rust testovi prolaze; u igri neprovjereno do novog installera.

## 2026-11-20 — Simulirani Ctrl-up poništio korisnikov držani Ctrl

- **Simptom:** u igri price kartica ne nestaje kad korisnik pusti Ctrl.
- **Dokaz:** `send_copy` je slao Ctrl-down, C, C-up, Ctrl-up dok korisnik fizički drži Ctrl (od Ctrl+D); `GetAsyncKeyState` prati logičko stanje uključujući poslane događaje, pa je nakon slanja Ctrl izgledao pušten, a hold watcher ga nikad nije vidio pritisnutog.
- **Uzrok:** simulirani pritisak nije uzeo u obzir tipke koje korisnik već drži.
- **Ispravak:** ako je Ctrl već dolje, šalje se samo C; Ctrl se šalje samo kad nije držan. Kartica prikazana nakon puštanja Ctrl-a zatvara se kad se kursor pomakne >120 px (Awakened ponašanje) umjesto da čeka timer.
- **Prevencija:** kod simuliranog unosa ne slati key-up za modifikatore koje korisnik drži.
- **Validacija:** Rust testovi prolaze; u igri neprovjereno do novog installera.
