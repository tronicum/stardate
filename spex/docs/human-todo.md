# human-todo.md — was Stefan von Hand korrigieren muss

*Stand: 2026-07-25 · Anlass: die Patentprüfung zu D1a und der Historiker-Review (Review 01 §4)*
*Gehört zu [`fugen/decisions.md`](fugen/decisions.md) D9/D10 — Beschaffung und Wikipedia-Arbeit sind Stefans Bahn, nicht die der Spec.*

Diese Liste ist bewusst nur das, was **von Hand in Wikipedia, Wikidata und
Commons** passieren muss. Kein Code, keine Engine. Reihenfolge nach
Dringlichkeit, nicht nach Aufwand.

---

## 0. Was sich geändert hat, in drei Sätzen

Die beiden britischen Batima-Schriften liegen jetzt mit ihren eigenen
Titelblattdaten vor. **GB 217243** (Priorität 6.6.1923, angemeldet 6.6.1924,
veröffentlicht 21.5.1925) heißt *„Improvements in toy building blocks"*,
Anmelder **„J. Girlot (assignee of L. Cousin)"**, und beschreibt **massive**
Blöcke aus *carton pierre*, die oben und unten ineinandergreifen. **GB 263865**
(Priorität 31.12.1925, angemeldet 31.12.1926, veröffentlicht 23.6.1927) heißt
*„Improvements in building blocks"*, nennt **nur J. Girlot**, und beschreibt
den Noppenstein: *„two rows of pegs a on one face and corresponding rows of
recesses b on the opposite face … the quarter brick having four pegs, the half
brick six, and the full brick eight."*

**Die Konsequenz:** Cousin ist als Erfinder des 1923er-Blocks durch die
Patentschrift gedeckt. Als Erfinder des **Noppensteins** ist er es nicht — die
Schrift, die die zwei Noppenreihen beschreibt, trägt seinen Namen nicht.

---

## 1. DRINGEND — die drei NL-Löschanträge (Frist ~07.08.2026)

`NL:Batima`, `NL:Hilary_Page`, `NL:Louis_Cousin_(uitvinder)`.

**Der Plan im alten Todo war, auf die Batima-Set-Lieferung zu warten und dann
Einspruch einzulegen. Das ist nicht die stärkste Verteidigung.** Ein Foto
eines Sets belegt, dass es das Set gab — es belegt keine der bestrittenen
Zuschreibungen. Was einen Löschantrag tatsächlich abwehrt, ist überprüfbare
Belegqualität. Und genau da haben die Artikel drei angreifbare Stellen, die
sich **jetzt** beheben lassen, und zwar zugunsten der Artikel:

1. **Zirkelbeleg.** In den Referenzen stehen `de.wikipedia.org/wiki/Batima` und
   `de.wikipedia.org/wiki/Louis_Cousin_(Erfinder)` als Quellen. Wikipedia als
   Beleg für Wikipedia ist [WP:CIRCULAR](https://en.wikipedia.org/wiki/Wikipedia:Verifiability#Wikipedia_and_sources_that_mirror_or_use_it)
   und wird in jeder Löschdiskussion als Erstes angegriffen — zu Recht.
   **Ersatzlos streichen und durch die Patentschriften ersetzen.**
2. **Unbelegtes Primärpatent.** `BE 311029` wird als Beleg zitiert, ist aber
   laut eurer eigenen Quellenlage noch nicht als Volltext beschafft
   („TODO: Volltext-PDF laden"). **Solange das PDF nicht vorliegt: GB 217243
   als Hauptbeleg führen** — das ist öffentlich einsehbar, nennt Cousin
   ausdrücklich und trägt dieselbe Priorität vom 6.6.1923.
3. **Erfundene Erfinderangabe in einer Zitatvorlage.** Im Wikitext steht
   `{{cite patent|country=FR|number=588985A|inventor=Cousin, Louis}}` —
   eure eigene Quellenlage hält aber fest: *„kein Erfinder benannt auf
   FR 588985, nur Anmelder LE BATIMA SOC"*. Eine Erfinderangabe in eine
   Patentzitatvorlage zu schreiben, die auf der Schrift nicht steht, ist
   genau die Sorte Fehler, die einen ganzen Artikel diskreditiert.
   **`inventor=` entfernen, `applicant=LE BATIMA SOC` setzen.**

Wenn diese drei Punkte vor dem Einspruch bereinigt sind, argumentiert der
Einspruch aus einer viel besseren Lage — und das Set-Foto kommt als Zugabe
obendrauf, nicht als Hauptargument.

---

## 2. Die Zuschreibung, in allen Sprachen

Betroffen: `Louis Cousin (Erfinder/inventor/inventeur/uitvinder/opfinder/发明家)`
in **DE · EN · FR · NL · DA · ZH** — und der `Batima`-Artikel in denselben
sechs Sprachen.

**Falsch (steht so drin):**

> „Er gilt als Schöpfer des Batima-Konstruktionsspielzeug-Systems — dem
> ältesten dokumentierten Stud-and-Socket-Klotzsystem der Geschichte."
> / „…the earliest documented interlocking stud-and-socket building block
> layout in history."

Das ist zweifach angreifbar: die Schrift, die Cousin nennt (GB 217243),
beschreibt **keine** Noppen, sondern massive ineinandergreifende Blöcke; und
die Schrift, die Noppen beschreibt (GB 263865), nennt **Girlot allein**.

**Richtig, und immer noch stark:**

> Louis Cousin' Klemmblock wird 1923 zum Patent angemeldet (GB 217243,
> „J. Girlot, assignee of L. Cousin", veröffentlicht 21. Mai 1925) — massive
> Blöcke mit oberseitigem und unterseitigem Formschluss. Der Baustein mit
> **zwei Reihen zu vier Noppen** erscheint 1927 unter Joseph Girlots Namen
> (GB 263865, veröffentlicht 23. Juni 1927). Beides ist Batima. Keines ist in
> Billund.

**Konkret zu ändern:**

- [ ] „stud-and-socket" / „Noppen" aus jedem Satz entfernen, der sich auf das
      **1923er** Patent bezieht.
- [ ] **GB 263865 als eigenen Beleg neu aufnehmen**, mit dem wörtlichen
      Abstract-Zitat („two rows of pegs…, the full brick eight") — das ist der
      stärkste Einzelbeleg, den das ganze Projekt hat, und er fehlt bisher in
      allen Artikeln.
- [ ] **Joseph Girlot als eigene Person führen** (mindestens als
      Rotlink/Erwähnung, ggf. eigener Artikel). Er ist auf beiden britischen
      Schriften der Anmelder und auf der wichtigeren der beiden der einzige
      Name.
- [ ] Materialangabe prüfen: die Artikel sagen **Galalith**, belegt mit einem
      Blog. GB 217243 sagt **carton pierre**. Beides kann stimmen
      (Patentmaterial vs. Produktionsmaterial) — dann muss es aber so
      dastehen, mit beiden Quellen, statt als eine unbelegte Tatsache.

---

## 3. Kiddicraft: das Datum ist falsch

Betroffen: `Kiddicraft` und `Hilary Page` in **DE · EN · FR · NL · DA · ZH**,
plus jede Zeitleiste, die „1939" nennt.

- [ ] **GB 529580** wird als *„Interlocking Building Cubes, 1939"* zitiert.
      Tatsächlich: Titel **„Improvements in toy building blocks"**,
      **eingereicht 17. April 1940**. Das Jahr in allen Zitatvorlagen und
      Fließtexten korrigieren.
- [ ] `GB 529580` und `GB 587206` werden mehrfach vermengt. 529580 (1940) und
      587206 (eingereicht 1944, erteilt 1947, *Self-Locking Building Bricks*)
      sind **zwei verschiedene Patente** und gehören sauber getrennt.
- [ ] Die Zeitleiste „Batima 1923 → Kiddicraft 1939 → Dehm 1946 → LEGO 1949"
      wird damit zu **1923 → 1927 → 1940 → 1946 → 1949**. Die 1927 fehlt
      überall und ist der eigentliche Kern.

---

## 4. Interlego v Tyco: falsches Gericht, falsche Aussage

Betroffen: überall dort, wo das Urteil zitiert wird — Artikel **und** Patent
Tracker.

- [ ] **„Hong Kong Supreme Court, Lego v. Tyco (1988)"** ist falsch. Richtig:
      **Judicial Committee of the Privy Council, 5. Mai 1988, [1988] UKPC 3 /
      [1989] AC 217**, im Berufungsverfahren aus Hongkong. Euer eigenes Todo
      hat den BAILII-Link bereits richtig — die Artikel haben ihn nicht.
- [ ] **„…belegt: LEGO-Design war direkte Kopie von Kiddicraft" /
      „rechtskräftig festgestellt"** ist keine tragfähige Wiedergabe. Das
      Urteil entscheidet über Urheberrecht an Zeichnungen und darüber, dass
      das Nachzeichnen mit Fleiß und Können keine neue Schutzfähigkeit
      begründet — **und LEGO hat es verloren.** Über Fisher Page als Erfinder
      sagt es nichts.
- [ ] Bessere Formulierung: *„der Fall, der die Zeichnungen beendet hat — nicht
      der Fall, der den Erfinder benannt hat."*

---

## 5. Der andere Louis Cousin

- [ ] In **jedem** Artikel zum Erfinder muss die Hatnote auf
      `Louis Cousin (Historiker, 1627–1707)` stehen und umgekehrt. Ist teils
      vorhanden, teils nicht.
- [ ] **Nirgends** einen Zusammenhang zwischen beiden andeuten. Im
      Bewegungs-Masterplan steht derzeit eine Ahnenreihe, die den Präsidenten
      der *Cour des Monnaies* (gest. 1707) mit dem Patentinhaber von 1923
      verbindet, samt einem „Ahne? Zufall?"-Einschub zu Albert
      Despature-Cousin. Das ist das Erste, was ein Historiker findet, und es
      beschädigt alles in seiner Nähe. **Ersatzlos streichen** — auch dort,
      wo es schon in Artikeltexte gewandert sein sollte.
- [ ] Nebenbei: die *Cour des Monnaies* wurde **1552** souveränes Gericht,
      nicht 1650.

---

## 6. Wikidata

- [ ] `Batima (Q110874585)`: `P17` Belgien, und die Patente als
      `P1246`/externe Identifikatoren mit **GB 217243** und **GB 263865**.
- [ ] Ein Item für **Joseph Girlot** anlegen und als `P61` (discoverer or
      inventor) an GB 263865 hängen — nicht an Cousin.
- [ ] `P144 (based on)` Kiddicraft → Batima: **erst setzen, wenn ein
      Sekundärbeleg dafür existiert.** Die Patentschriften belegen die
      Zeitfolge, nicht die Ableitung. Ohne Beleg wird die Aussage
      zurückgesetzt und schwächt die anderen.
- [ ] Alle Sitelinks der sechs Sprachen nachtragen (steht schon im alten Todo).

---

## 7. Commons

- [ ] Die beiden vorbereiteten Uploads laufen unverändert weiter
      (`Batima-bricks-CC0.jpg`, `FR588985A-…-1924.jpg`).
- [ ] **Beschreibungstext anpassen:** „Earliest documented stud-and-socket
      building block system" ist nach Punkt 2 nicht haltbar. Stattdessen:
      *„Batima construction toy bricks with original instruction manual.
      Earliest documented interlocking building block system of the Batima
      series (GB 217243, priority 1923); the two-rows-of-four-studs brick
      follows in GB 263865, published 1927."*
- [ ] **Zusätzlich hochladen**, sobald beschafft: die Titelblätter von
      GB 217243 und GB 263865. Beide sind über 100 Jahre alt bzw. amtliche
      Veröffentlichungen — Lizenzstatus jeweils prüfen, nicht annehmen.

---

## 8. Patent Tracker (research.iunctura.org)

- [ ] GB 263865 als eigene Zeile aufnehmen — es fehlt, obwohl eure eigene
      Quellenlage es „das wichtigste Dokument" nennt.
- [ ] Das Urteil nach Punkt 4 korrigieren.
- [ ] Kiddicraft-Jahr nach Punkt 3 korrigieren.
- [ ] **Wichtig für die Kette:** der Tracker ist bereits auf IPFS gepinnt und
      per OpenTimestamps verankert. Eine Korrektur erzeugt eine **neue** CID.
      Die alte nicht stillschweigend ersetzen — beide Fassungen führen, mit
      Datum und Änderungsgrund. Ein Archiv, das seine eigenen Korrekturen
      verschweigt, ist kein Archiv.

---

## 9. Was daraus für das Werk folgt (nur zur Info, kein Todo)

Die korrigierten Fassungen sind bereits in
[`fugen/screenplay.md`](fugen/screenplay.md) §5 eingearbeitet: Untertitel
`FR 588985 · Le Batima Soc. · c. 1924` statt `BE 311029 · 1923`,
`GB 529580 · filed 17 April 1940` statt 1939, und das Urteil mit der
korrigierten Bildunterschrift. Die Premiere hängt an GB 263865s
Veröffentlichung: **Mittwoch, 23. Juni 2027.**
