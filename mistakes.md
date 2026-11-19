# Potvrđene greške i kako ih ne ponoviti

Ovaj zapis vodi razvojni agent. Dodavati samo opažene greške ili dokazive nedostatke, uz ispravak i status provjere. Ne pretvarati pretpostavke u potvrđene uzroke. Datumi/vremena odnose se na Europe/Zagreb.

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
