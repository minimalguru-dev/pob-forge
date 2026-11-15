# Potvrđene greške i kako ih ne ponoviti

Ovaj zapis vodi razvojni agent. Dodavati samo opažene greške ili dokazive nedostatke, uz ispravak i status provjere. Ne pretvarati pretpostavke u potvrđene uzroke. Datumi/vremena odnose se na Europe/Zagreb.

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
- **Validacija:** lokalni diskriminacijski test potvrđuje uzrok; idući Windows CI potvrđuje cijeli sync.

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
- **Validacija:** integration test eksplicitno očekuje `false` za `conditionKilledRecently` i uspoređuje boss rezultate s direktnim PoB izračunom; native izvršavanje još čeka CI.

## 2026-11-15 — Doseg Lua lokalnih varijabli između modula

- **Nalaz u pregledu prije prvog commita:** novi modul referencirao je `GAME`, ali je ta varijabla lokalna u `bridge.lua`.
- **Ispravak:** game provjeru čitati preko proslijeđenog `methods.version().game`.
- **Prevencija:** kod izdvajanja modula eksplicitno proslijediti ovisnosti; ne pretpostavljati da je bridge-local stanje globalno.
- **Validacija:** statički pregled ispravljen; stvarni engine test čeka native CI.

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
