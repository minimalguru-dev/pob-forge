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
