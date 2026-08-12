# M75 — `spex-flag`: was gemessen wurde, und was dabei herauskam

*12. August 2026 · Tier A: BE, DK, GB*

## Was gebaut ist

`crates/spex-flag/` — `model` (das Konstruktionsblatt), `raster` (das Blatt in
Zellen), `quantize` (Zellen auf echte Steine, mit dem ΔE, den es gekostet hat).
`spex flag <iso2> --width-studs N -o <rezept.json>`. Drei Blätter in `flags/`.

**Abweichung von der Spec, hier festgehalten statt verschwiegen:** die Spec
verlangt `emit_flag_ldr`. Es gibt keinen. `spex_build::Mosaic` macht aus einem
Raster von Farbcodes bereits eine Kachel pro Zelle auf dem echten Stud-Raster —
`feld.json` (3 600 Module, A3-S05) ist daraus gebaut. Ein zweiter Emitter wäre
ein zweiter Satz Entscheidungen über dieselbe Sache, und die beiden würden
irgendwann auseinanderlaufen. `spex flag` schreibt daher ein **Rezept**, und
die `.ldr` fällt aus derselben Pipeline wie bei jeder anderen Szene des Stücks.

## Die Abnahmekriterien

| | Stand |
|---|---|
| **AC1** Dannebrog-Kreuz, in Studs, auf einen Stud genau gegen die veröffentlichten Brüche | **erfüllt**, und zusätzlich exakt: bei 37 × 28 Studs kommen 12/4/21 und 12/4/12 ohne jede Rundung heraus |
| **AC2** Union Flag, echtes asymmetrisches Schrägkreuz | **erfüllt** — an beiden Seiten gemessen, breites Weiß oben am Liek, schmales oben am fliegenden Ende, jeweils gegen 3 / 2 / 1 von 30 Höheneinheiten |
| **AC3** `max_delta_e` für jede Flagge festgehalten | **erfüllt**, siehe Tabelle |
| **AC4** jedes Blatt zitiert seine Quelle | **erfüllt**, als Test |
| **Leiter Sprosse 5** Kontaktbogen, von einem Menschen durchzusehen | erzeugt (`scripts/flags/contact-sheet.py`) — **die Durchsicht steht aus** |

CIEDE2000 ist gegen alle zehn Referenzpaare aus Sharma, Wu & Dalal (2005) auf
1e-4 geprüft. Das ist die Tabelle, die es gibt, weil Implementierungen dieser
Formel den Hue-Rotation-Term falsch machen.

## Die Zahlen

Palette: 102 Farben — deckend, `solid`, und **im Abschnitt „LDraw Solid
Colours" der echten Datei**.

| | breiteste Abweichung | Zuordnung |
|---|---|---|
| **DK** | **ΔE 9,53** | Rot → `4 Red` (9,53), Weiß → `15 White` (2,22) |
| **GB** | **ΔE 9,53** | Rot → `4 Red` (9,53), Blau → `272 Dark_Blue` (7,16), Weiß → `15 White` (2,22) |
| **BE** | **ΔE 12,72 — über der Schwelle** | Schwarz → `0 Black` (12,72), Rot → `123 Dark_Salmon` (9,01), Gelb → `14 Yellow` (5,30) |

## Der Befund, der die Arbeit wert war

**Die belgische Flagge lässt sich nicht bauen, und das Schwarz ist schuld.**

LDraw `0 Black` ist `#1B2A34` — ein sehr dunkles Blaugrau, kein Schwarz. Gegen
das reine Schwarz der Protokollvorschrift sind das ΔE 12,72, über der Schwelle
von 12, die `phase4-kit.md` selbst setzt. Im Kontaktbogen sieht man es: die
Bahn am Liek ist erkennbar blau. Dazu kommt ein Rot, das als `Dark_Salmon`
landet, weil `4 Red` zu tief ist.

Das ist kein Fehler, den man wegräumen sollte. Das Drehbuch sagt es selbst:
*„a flag that cannot be built in the available palette is this thesis's best
counter-evidence, and showing it costs nothing."* Belgien ist genau dieser
Fall, und es ist auch noch das Land des ersten Postillen.

## Ein Defekt, den erst die Flagge sichtbar gemacht hat

Die erste belgische Flagge kam in **`30006 Modulex_Ochre_Yellow`** heraus.
Modulex ist eine andere Produktlinie in einem anderen Maßstab; ihre Einträge in
`LDConfig.ldr` sind gewöhnliche deckende `!COLOUR`-Zeilen ohne
Material-Schlüsselwort, also `Finish::Solid` wie jeder Stein. Der Quantisierer
hat eine gewählt, weil sie in Lab am nächsten lag — eine Farbe, aus der niemand
eine Flagge bauen kann.

Das Einzige in der echten Datei, das sie unterscheidet, sind ihre eigenen
Abschnittsüberschriften (`0 // LDraw Modulex Colours`). `spex-ldraw` trägt sie
jetzt als `LdrawColor::section`. Die Palette schrumpfte damit von 147 auf 102.

## Was eine Flagge ihre natürliche Breite ist

Bei 48 Studs ist der Dannebrog-Arm in der einen Richtung 5 Studs und in der
anderen 6 — beides ist 5,14 auf das nächste Baubare gerundet, aber sichtbar
ungleich. Bei **37 × 28** geht jede Teilung exakt auf. Die natürlichen Breiten:

| | Verhältnis | natürlich |
|---|---|---|
| DK | 28:37 | 37 × 28 (und Vielfache) |
| BE | 13:15 | 45 × 39 |
| GB | 1:2 | 60 × 30 |

Für den Atlas heißt das: die Breite gehört zur Flagge, nicht zum Aufruf.

## Was offen ist

- **Die Durchsicht des Kontaktbogens** — Sprosse 5 ist Pflicht und nicht
  delegierbar.
- **Belgien**: Ausschluss aus dem Atlas, oder aufnehmen und die ΔE-Zahl auf die
  Leinwand? Das Drehbuch will Letzteres, aber es ist Stefans Entscheidung.
- **Der Union Flag bei 3:5**: dort werden die inneren Spitzen zweier Arme des
  Patrickkreuzes gekappt. Das Blatt hier ist 1:2, wo das nicht auftritt; ein
  3:5-Blatt bräuchte das Element und müsste es sagen statt nähern.
- **„derzeit produzierte Farben"** ist nicht umgesetzt. `LDConfig.ldr` führt
  keinen Produktionsstatus, und ihn aus der Datei zu erraten hieße, Daten zu
  erfinden. Das braucht eine echte Quelle.
