# Kuratierung des Atlas — Entwurf zur Durchsicht

*12. August 2026 · M73 · aus `scripts/heritage-data/curation.draft.json`*

Der Schnappschuss hat **1 411 Stätten** (Wikidata, CC0, 12. August 2026).
Dieser Entwurf klassifiziert davon 48. Alle übrigen 1 363 sind **unklassifiziert
und damit ausgeschlossen** — der Filter fällt geschlossen aus.

`spex heritage-list --buildable` **verweigert die Arbeit**, solange `reviewed`
und `reviewer` leer sind. Erst deine Unterschrift macht aus dem Entwurf eine
Kuratierung.

---

## 1. Ausschlussliste — 8 Stätten

Nicht verhandelbar, und der Teil, der ausdrücklich nicht delegierbar ist.
Die ersten fünf nennt die Spec; die letzten drei habe ich beim Lesen des
Schnappschusses gefunden. Dass sie dort standen und nicht in der Spec, ist
das Argument dafür, die Liste gegen echte Daten zu entwerfen.

- **Auschwitz** (`Q7341`) — The German Nazi concentration and extermination camp. A site of industrialised mass murder is not a subject for toy bricks, under any framing.
- **Hiroshima Peace Memorial** (`Q231140`) — The ruin left by the atomic bombing, preserved as it stood. The building is a record of the deaths beneath it.
- **Gorée** (`Q244347`) — A centre of the Atlantic slave trade, and a memorial to it.
- **Robben Island** (`Q192493`) — A political prison; the site is inscribed for what was done to the people held there.
- **Bikini Atoll** (`Q152225`) — A nuclear test site: the population was displaced and the atoll is still uninhabitable.
- **Memorial sites of the Genocide: Nyamata, Murambi, Gisozi and Bisesero** (`Q56293252`) — The Rwandan genocide memorials at Nyamata, Murambi, Gisozi and Bisesero. Not named in the spec; found in the snapshot, and it belongs here for exactly the same reason Auschwitz does.
- **Murambi Genocide Memorial** (`Q5964053`) — A second Rwandan record in the snapshot; excluded on its own QID so a merge or split upstream cannot quietly re-admit it.
- **Cambodian Memorial Sites: From centres of repression to places of peace and reflection** (`Q134721311`) — Centres of repression under the Khmer Rouge. Not named in the spec; found in the snapshot.

## 2. Aktive Kultstätten — 10 von 40, per Voreinstellung ausgeschlossen

Die Spec schließt aktive Kultstätten voreingestellt aus; Aufnahme braucht eine
ausdrückliche, festgehaltene Entscheidung. Ich habe sie im Entwurf gelassen,
damit die Entscheidung etwas hat, worüber sie fällt — aber sie sind hier
gesondert aufgeführt, weil sie **einzeln** entschieden werden müssen.

- **Historic Sanctuary of Machu Picchu** (`Q3331815`, Tier B) — the Intihuatana is a ritual stone, not an active place of worship.
- **Borobudur Temple Compounds** (`Q29070`, Tier B) — an active Buddhist pilgrimage site.
- **Cologne Cathedral** (`Q4176`, Tier B) — an active cathedral.
- **Angkor** (`Q2397751`, Tier C) — parts are in active Buddhist use.
- **Wieliczka and Bochnia Royal Salt Mines** (`Q15240748`, Tier C) — the Chapel of St Kinga holds services.
- **Mont-Saint-Michel and its Bay** (`Q17117964`, Tier C) — an active monastic community.
- **Timbuktu** (`Q9427`, Tier C) — active mosques.
- **Bagan** (`Q29317`, Tier C) — active.
- **Vatican City** (`Q237`, Tier C) — the entire state.
- **Völklingen Ironworks** (`Q127754`, Tier C) — no.

## 3. Baubare Stätten — 40 mit geschriebener Begründung

Jede Begründung sagt, was die **sieben vorhandenen Primitive** tatsächlich
bauen würden — `Wall`, `Column`, `Arch`, `Colonnade`, `Dome`, `Stair`,
`Mosaic`. Eine Stätte, die ein achtes Primitiv braucht, ist eine Stätte, die
dieser Atlas nicht ehrlich behaupten kann.

### Tier A — 3

- **Stonehenge** (`Q39671`) — Trilithons and a continuous lintel ring: Column plus a lintel course. Already built as recipes/stonehenge.json, 28 090 instances from cited stone dimensions.
- **Grand-Place** (`Q215429`) — A closed square of guildhall facades: Wall for the elevations, Colonnade at ground level, Stair to the Town Hall. Gables are stepped Wall courses.
- **Jelling Heritage Site** (`Q4993586`) — Two barrows as Dome, two runestones as small blocks, one church as Wall plus roof. The smallest Tier A site and the clearest test of the vocabulary.

### Tier B — 9

- **The Great Wall** (`Q65961372`) — Wall and Stair, repeated -- the primitive is the site's name. Watchtowers are Column plus Wall.
- **Memphis and its Necropolis – the Pyramid Fields from Giza to Dahshur** (`Q1175856`) — The Giza pyramids: the Pyramid primitive, stepped in whole courses, which is what a brick pyramid honestly is.
- **Historic Centre of Rome, the Properties of the Holy See in that City Enjoying Extraterritorial Rights and San Paolo Fuori le Mura** (`Q18448486`) — The Colosseum is Arch on Colonnade, four storeys, which is precisely what the building is. Already partly built as recipes/rom.json.
- **Acropolis of Athens** (`Q131013`) — The Parthenon is Colonnade plus Stair plus a pediment of stepped Wall courses.
- **Taj Mahal** (`Q9141`) — Dome on a plinth, four Columns as minarets, Arch iwans on each face.
- **Historic Sanctuary of Machu Picchu** (`Q3331815`) ⚠️ — Terraces as Stair, buildings as Wall.
- **Borobudur Temple Compounds** (`Q29070`) ⚠️ — Stepped terraces (Stair) with stupas (Dome).
- **Cologne Cathedral** (`Q4176`) ⚠️ — Twin towers as Column, nave as Wall, the whole thing as Arch.
- **Pont du Gard** (`Q189764`) — Three tiers of Arch. Nothing else. The cleanest possible demonstration of one primitive.

### Tier C — 28

- **Petra** (`Q5788`) — Rock-cut facades as Colonnade and Arch against a Wall face.
- **Angkor** (`Q2397751`) ⚠️ — Towers as stepped Column, galleries as Colonnade, causeways as Wall.
- **Chichen Itza** (`Q5859`) — El Castillo is the Pyramid primitive with Stair on all four faces.
- **Persepolis** (`Q129072`) — The Apadana is Colonnade on a Stair-approached terrace.
- **Mesa Verde National Park** (`Q237128`) — Cliff Palace as stacked Wall and Column under an overhang.
- **Bauhaus and its Sites in Weimar, Dessau and Bernau** (`Q14863645`) — Wall and glazing grid as Mosaic. A modular building rendered by a modular system, which is the thesis stated twice.
- **Zollverein Coal Mine Industrial Complex** (`Q122026`) — Shaft 12's headframe as Column and truss, halls as Wall.
- **Speicherstadt and Kontorhaus District with Chilehaus** (`Q20644172`) — Brick warehouses: Wall and Arch, and the single most literally brick-built site on the list.
- **Kronborg Castle** (`Q189358`) — Bastioned castle: Wall, Column, Stair. Hamlet's Elsinore, and Danish, which the Postillen make relevant.
- **Belfries of Belgium and France** (`Q750675`) — The Bruges belfry as a stepped Column. Belgian, likewise.
- **Wieliczka and Bochnia Royal Salt Mines** (`Q15240748`) ⚠️ — Timbered chambers as Wall and Column.
- **Alhambra, Generalife and Albayzín, Granada** (`Q9603543`) — Courts as Colonnade, the Court of the Lions as Column repeated, Arch throughout.
- **Mont-Saint-Michel and its Bay** (`Q17117964`) ⚠️ — The abbey as Wall on a Stair-terraced rock.
- **Sydney Opera House** (`Q45178`) — The shells are the one site here the vocabulary genuinely strains at -- a Dome is a hemisphere and these are not. Drafted as a candidate for exactly that reason: if it cannot be built honestly it should be dropped and said so.
- **Brasília** (`Q2844`) — Niemeyer's Congress: Dome, inverted Dome, twin Wall slabs.
- **Rapa Nui National Park** (`Q1763364`) — Moai as Column with a pukao course. Ahu platforms as Wall.
- **Timbuktu** (`Q9427`) ⚠️ — Djinguereber and Sankore as Wall with projecting toron.
- **Bagan** (`Q29317`) ⚠️ — Stupas as Dome on stepped Stair terraces, repeated at scale.
- **Himeji Castle** (`Q188754`) — The keep as stacked Wall with tiered roofs; roofs as stepped courses.
- **Vatican City** (`Q237`) ⚠️ — Bernini's colonnade is Colonnade, four columns deep, which is the primitive's definition.
- **Old Town of Segovia and its Aqueduct** (`Q15728389`) — The aqueduct is Arch on Arch, 167 of them.
- **Ironbridge Gorge** (`Q647958`) — The bridge as Arch in cast iron -- the first of its kind, and the industrial rhyme to Act III.
- **Völklingen Ironworks** (`Q127754`) ⚠️ — Blast furnaces as Column and Stair.
- **Sigiriya** (`Q272153`) — The rock as a mass, the water gardens as Mosaic, the lion staircase as Stair.
- **Archaeological Sites of the Island of Meroe** (`Q3962345`) — Nubian pyramids: the Pyramid primitive, steeper than Giza's, repeated across a field.
- **Great Zimbabwe** (`Q209217`) — Dry-stone enclosures: Wall, in the most literal sense on this list -- coursed, mortarless, modular.
- **Fujian Tulou sites** (`Q718001`) — Circular earthen dwellings: Wall bent into a ring, which is a real test of whether the primitive can curve.
- **Citadel, Ancient City and Fortress Buildings of Derbent** (`Q64763166`) — Wall running from the citadel to the sea.

---

## Was mir dabei aufgefallen ist

**Sydney Opera House ist der ehrliche Grenzfall.** Ein `Dome` ist eine
Halbkugel, und die Schalen sind es nicht. Ich habe es genau deshalb in den
Entwurf genommen: wenn es sich nicht ehrlich bauen lässt, soll es rausfallen
und das gesagt werden — nicht stillschweigend genähert.

**Speicherstadt ist die literalste Stätte der Liste.** Backsteinlager: `Wall`
und `Arch`, nichts sonst. Wenn eine Stätte diese These beweist, dann die.

**Vatikanstadt ist als Ganzes eine Kultstätte.** Berninis Kolonnade ist die
Definition des Primitivs, und der Staat ist der Heilige Stuhl. Das ist keine
technische Frage.

