# Backlog — Ideen, die noch nicht entschieden sind

*Nicht Teil des Meilensteinplans. Regel 8 aus [`README.md`](README.md): neue Ideen kommen hierher, nicht in den laufenden Meilenstein.*

Jeder Eintrag hält fest, **was** die Idee ist, **warum sie stark ist**, und
**was sie kosten würde** — damit eine spätere Sitzung sie aufgreifen kann,
ohne sie neu erfinden zu müssen. Nichts hier ist beschlossen.

---

## B1 — *Fügung*: die dritte Bedeutung

**Stefan, 2026-07-25: „ist eine Fügung des Himmels".**

Das Werk trägt den Doppelnamen *Fuge* — die Naht zwischen zwei Steinen, und
die musikalische Form. **Es gibt eine dritte, und sie ist die interessanteste
von allen:** *fügen* → *die Fügung*. Im Deutschen heißt dasselbe Wort, das
das Zusammensetzen bezeichnet, auch **das, was das Schicksal anordnet**. Eine
*Fügung des Himmels* ist genau das, was das Werk bestreitet.

**Warum das mehr ist als ein Wortspiel.** Die These lautet bisher: Zählen,
Bauen und Rechnen sind derselbe Instinkt. Das ist wahr und deshalb ein wenig
zahm — man kann kaum widersprechen. Die dritte Bedeutung schärft sie zu einer
Behauptung, der man widersprechen *kann*:

> Was wie Fügung aussieht, ist Fertigung.

Fünftausend Jahre Normmaß erscheinen im Rückblick als Notwendigkeit, als
hätte es so kommen müssen — und jede einzelne Stufe war eine Entscheidung
von jemandem, der etwas herstellen wollte. Das Werk hält das Wort hoch und
zeigt, dass es beides bedeutet, und dass nur eine der beiden Bedeutungen
stimmt. Das trifft sich mit der bereits beschlossenen Korrektur aus
[`screenplay.md`](screenplay.md) §5: *„Dies ist eine Metapher. So bauen
Menschen."*

**Wo es hingehen könnte** — nichts davon ist entschieden:
- als Titelkarte oder Schlusszeile, in der das Wort einmal in allen drei
  Bedeutungen steht;
- als Akt-IV-Text, dort wo das Werk sonst am ehesten in Verkündigung kippt;
- gar nicht sichtbar, sondern nur als Regieprinzip für den Tonfall.

**Offen:** übersetzbar ist es nicht. *Joint / fugue / providence* fällt im
Englischen auseinander, und ein Werk, dessen Pointe nur auf Deutsch
funktioniert, muss das entweder aushalten oder erklären. Beides ist möglich,
keins ist gratis.

---

## B2 — Die Fuge pulsiert im Raum

**Stefan, 2026-07-25: „die Fuge pulsiert im Raum — wie, schauen wir mal."**

Bisher ist Klang im Werk Stereo aus dem Rechner. Die Idee: der Puls ist nicht
etwas, das man hört, sondern etwas, das **im Raum steht** — für die
Installationsfassung, nicht für den Browserschnitt.

**Warum es passt:** die Fuge ist eine Verbindung zwischen zwei Dingen. Wenn
das Stück im Raum spielt, ist der Raum das zweite Ding. Ein Puls, der von
einer Wand zur anderen wandert, ist buchstäblich eine Fuge zwischen zwei
Punkten — und die vier Stimmen des Kontrapunkts sind das naheliegendste
Material dafür, das es gibt: **vier Stimmen, vier Positionen.**

**Was es technisch bedeutet** (nur Skizze, nichts geprüft):
- Im Browser: WebAudio kann `PannerNode`/HRTF und Ambisonics erster Ordnung.
  Kostet wenig und klingt über Kopfhörer sofort anders.
- Im Raum: vier Kanäle statt zwei, eine Stimme pro Lautsprecher, der Puls
  wandert. Das ist kein Browser-Thema mehr, sondern Installationsaufbau —
  Interface, Verkabelung, Einmessen vor Ort.
- Der Kick am Schluss wäre dann das einzige Ereignis, bei dem alle vier
  Positionen gleichzeitig sprechen. Das ist eine gute Idee und genau deshalb
  einen eigenen Termin wert.

**Was es kosten würde:** M69/M71 bleiben unberührt, solange das Stereo-Rendering
die Referenz bleibt. Eine Mehrkanalfassung ist ein **eigener Meilenstein nach
der Premiere**, kein Zusatz zu Phase 3 — sonst wandert der Aufwand in die
Phase, deren größtes Risiko ohnehin schon eine Geschmacksfrage ist.

**Offen:** gibt es überhaupt einen Raum? Ohne zugesagten Ausstellungsort ist
das eine Lösung ohne Problem. Frühestens nach D5s Antwort sinnvoll zu
verfolgen.

---

## B4 — MIDI hinaus: eine mechanische Aufführung

Aufgekommen aus Stefans Frage, ob die Musik nicht per MIDI aufgeführt werde.
Im Browser: nein, denn dort gibt es keinen Klangerzeuger — MIDI ist das
Ereignismodell, geklungen wird per WebAudio-Synthese
([`phase3-audio.md`](phase3-audio.md)).

**Aber die Web MIDI API kann MIDI *hinaus*schicken.** Für die
Installationsfassung heißt das: das Stück könnte ein echtes Instrument
spielen statt eines Lautsprechers. Ein Synthesizer, ein Disklavier, eine
Orgel.

**Warum das mehr wäre als eine Spielerei.** Das Werk handelt von einem
genormten Modul, das seit hundert Jahren unverändert funktioniert, und von
der Behauptung, dass Fügung in Wahrheit Fertigung ist. Eine Fuge, die von
einer Mechanik gespielt wird — Hämmer, Ventile, Luft —, führt genau das vor,
statt es zu behaupten. Und die Orgel ist ohnehin das Instrument der Form.

**Was es kostet:** im Browser fast nichts (die Ereignisse liegen schon in
MIDI-Semantik vor, es fehlt nur der Ausgang). Vor Ort alles: ein Instrument,
das man ansteuern darf, Latenzabgleich gegen die Bildspur, ein Ausfallplan,
wenn die Mechanik klemmt. Gehört zu B2, nicht zum Webschnitt.

**Offen:** dasselbe wie bei B2 — gibt es einen Raum, und steht dort etwas,
das MIDI annimmt?

---

## B3 — Die Schleife, die etwas behält

Aus Review 01 §3, als Kompromiss aufgenommen und hier als Vollversion
geparkt: der Creative Director wollte die Schleife **brechen** — pro Zyklus
ein Stein, der nicht zurückkommt. Beschlossen wurde die schwächere Fassung
(die dunkle Zone der Weltplatte bleibt, das Pixel bleibt identisch), weil
Pixelgleichheit für die Endlosfassung und die On-Chain-Edition trägt.

Sollte sich herausstellen, dass die Determinismus-Anforderung lockerer ist
als angenommen, ist die stärkere Fassung die bessere Arbeit. Nicht vergessen,
sondern bewusst zurückgestellt.
