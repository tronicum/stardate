# The screenplay — *Die Geschichtliche Matrix*

*The canonical cut, authored in bars. Rev 3 supersedes rev 1's second-based timings.*

**Part of [`docs/fugen/`](README.md) — the Fugen Engine implementation spec, rev 3.**
Read [`README.md`](README.md) first: it carries the working rules, the scope decision and the milestone index.
Review record and the reasoning behind every change here: [`../FUGEN-ENGINE-REVIEW-01.md`](../FUGEN-ENGINE-REVIEW-01.md) §3 and §4.

---

> *"Wir bauen, um zu verstehen. Wir archivieren, um zu bewahren. Wir fügen,
> um zu sein."*

## 0. What changed in rev 3, and why

Rev 1 authored the piece in seconds. At 84 bpm in 4/4 a bar is 20/7 s =
**2.857142857 s**, and almost none of rev 1's durations were bar multiples —
so the screenplay's own Rule 2 ("every cut lands on a bar line") was
unsatisfiable, and the resolver's beat-snapping step would have silently
moved every cut. Rev 1 also said "336 bars", where 240 s at 84 bpm is 336
*beats* = **84 bars**.

**The piece is now authored in bars. Seconds are derived, never authored.**

Two reviewers — a screenwriter and a creative director, working
independently — reached the same diagnosis: *the film has no memory between
shots.* Everything accumulates, nothing is ever lost, so the piece can be
agreed with but not felt. Five changes follow from that, and they are the
substance of rev 3:

1. **Act budget 17 / 20 / 20 / 6 / 21 bars** instead of rev 1's flat
   60 / 60 / 60 s. The film should get longer as it gets more urgent; rev 1
   gave the setup act the same weight as the payoff act.
2. **A4-S01b "Der letzte Stein" is added** (2 bars). The single change that
   turns an argument into an arc.
3. **The monolith becomes a continuity object** — visibly present at the edge
   of frame once per act, so there is something that persists and can be lost.
4. **The Atlas accumulates** instead of resetting between sites.
5. **A1-S03 is the centre of the piece**, not the clutch macro, and its
   music enters *after* the image lands rather than with it.

A sixth change came from the historian's review: several dated claims were
wrong. Every caption below is the corrected one. See §5.

## 1. The grid

| | |
|---|---|
| Tempo | **84 bpm**, 4/4 |
| Bar | **20/7 s = 2.857142857 s** |
| Canonical cut | **84 bars = 240.000 s exactly** |
| Mode | Dorian on D |
| Voices | 4 (S / A / T / B) + a pulse layer from Act IV |

Every shot duration below is an integer bar count, except the Kick, which is
**½ bar = 2 beats = 1.428571 s** and is `scaling: 'fixed'` in every cut, at
every duration, forever. A resolver that stretches the Kick is broken.

## 2. The palette

| Name | Value | Where |
|---|---|---|
| Terrakotta | LDraw 70 *Reddish Brown* family / `#B5704E` | Uruk brick, the bulla, Act II — and the last stone |
| Elektrum | LDraw 297 *Pearl Gold* / `#D4AF37` | the Sardis coin, Act II |
| Terminalgrün | `#00E633` | the token field, Act IV, and the final pixel |
| Schwarz | LDraw 0 *Black* `#1B2A34` | the monolith, Act I |
| Patentweiß | `#F2EFE6` (cooled from rev 1) | the studio of Act III, patent-drawing overlays |
| Steingrau | LDraw 72 *Dark Bluish Gray* | Stonehenge and the Atlas's stone sites |

The technical-art review's warning stands: Terrakotta, Elektrum, Patentweiß
and Steingrau all sit in one warm-to-neutral band, so **Acts II and III must
separate by value and material, not hue** — cool the Patentweiß, drop
Elektrum's diffuse and raise its specular. And a `#1B2A34` monolith on a
near-black field lit only by a rim (A1-S06) is carried entirely by its
`#595959` edge lines, i.e. a grey wireframe. The background is a vertical
gradient `#05070a → #0d1219`, not flat black, so the monolith has something
to silhouette against.

## 3. The three rules

1. **Every object is made of real modules.** Nothing in frame is geometry
   that is not a real LDraw part, except light, text, and the final pixel.
2. **Every cut lands on a bar line.** Now true, because §1.
3. **The piece begins and ends on the same single green pixel.** The loop is
   the argument — but see §6 on what the loop is allowed to remember.

---

## 4. The shot list

### ACT I — ARCHÄOLOGIE DER FUGE · bars 0–17 · 0:00.000–0:48.571

*The individual natural object. Counting has not been invented yet.*

| Shot | Bars | In | Dur | Scaling | Tier | Content |
|---|---|---|---|---|---|---|
| **A1-S01** | 0–2 | 0:00.000 | 2 | fixed | 1 | **Black.** One point, Terminalgrün, at frame centre, one device pixel. It pulses once, slowly. Camera static. Audio: silence, then a 55 Hz sine fades up over 4 s. *This frame is byte-identical to the last frame of the piece.* |
| **A1-S02** | 2–4 | 0:05.714 | 2 | stretch (2–7) | 1 | The point **swells into a swarm** — the existing point renderer, ~3 000 points sampled from `3005.dat`'s real surface. It has no shape yet; it is a cloud. Camera pushes in. Rev 1 gave this 8 s; a formless swarm has no second beat. |
| **A1-S03** | 4–7 | 0:11.429 | 3 | stretch (2–11) | 1 | **The crossfade, and the centre of the piece.** The swarm collapses onto the real mesh of a single 1×1 brick, Black. **The edge lines arrive in one frame — not a fade.** Legibility is a threshold event: a statistical cloud becomes a thing that can be counted. Then **hold silence for one bar.** Voice 1 (alto) enters with the subject *after* the image has landed, alone. Then the brick makes one full revolution at constant angular velocity. |
| **A1-S04** | 7–11 | 0:20.000 | 4 | stretch (3–14) | 1 | **The assembly.** Nine real parts (7× `3010.dat` + 1× `3710.dat` + **1× `2431.dat`**, Black) fly in from the scattered start — `FLOAT_HEIGHT_LDU` 420, `SCATTER_RADIUS_LDU` 260, per-placement splitmix seed, the existing constants — eased `cubicInOut`, staggered by real build step, settling into `ldraw-scenes/monolith.ldr`. The top course is a TILE and not a plate: same 8 LDU, no studs, so the stack reads as one smooth slab from above rather than as a stack with a lid — and swapping rather than adding keeps the height at 184 LDU = 73.6 mm and the ratio at 1 : 4 : 9.20. (1 : 4 : 9 exactly is not constructible from 1×4 bricks and plates: 24a + 8b = 180 means 6a + 2b = 45, even on the left and odd on the right. 176 LDU and 184 LDU bracket it, both off by 0.20 — which is the piece's own subject and not a defect in the file.) Each landing is a tile-click accent. Voice 2 (soprano) enters with the tonal answer as the first part lands. |
| **A1-S05** | 11–14 | 0:31.429 | 3 | stretch (2–32) | 1 | **The monolith stands.** Camera orbits 180° from low, the object filling 80 % of frame height. Hairline HUD, lower right: `1 : 4 : 9.20 — 73.6 mm — 9 real parts`. The key light rakes across the studs so the module count is *countable*. Voice 3 (tenor) enters. |
| **A1-S06** | 14–17 | 0:40.000 | 3 | stretch (2–42) | 1 | **Stonehenge rises** behind the monolith — `heritage/stonehenge.ldr`, Steingrau, materialising from the ground up at a scale that reveals the monolith was small all along. Camera dollies back. Voice 4 (bass) enters; the exposition is complete in four voices. Caption, if any: **c. 2500 BCE** — the sarsens, lintels, mortices and tenons, not the 3000 BCE ditch. |

**Direction.** Nothing here is fast, but rev 1 was slack: 17 bars, not 21.
If a viewer is bored at bar 14 the act is working; if they are bored at bar 4,
A1-S02 is still too long and gets re-weighted, not re-cut.

---

### ACT II — DER CORE STANDARD · bars 17–37 · 0:48.571–1:45.714

*The module becomes an administrative fact. Value becomes portable.*

| Shot | Bars | In | Dur | Scaling | Tier | Content |
|---|---|---|---|---|---|---|
| **A2-S01** | 17–21 | 0:48.571 | 4 | stretch (2–21) | 1 | **Uruk, ~3500 BCE.** A wall builds itself: `Wall{bond: Running}` in Terrakotta, one course per beat, 12 courses in 3 bars, then settle. The camera holds absolutely still — the first static shot of the piece, so the construction moves and nothing else does. Caption: **"the brick is sized to the hand that carries it."** *Not* "the first standardised brick": moulded brick predates Uruk by roughly four thousand years. |
| **A2-S02** | 21–25 | 1:00.000 | 4 | stretch (2–25) | 1 | **One brick leaves the wall**, leaving a legible gap, and floats to camera. As it rotates it *becomes* a clay bulla — a corbelled `Dome` of Terrakotta bricks growing around it while the source brick dissolves. Not a morph: a rebuild. The module becomes a container. **The monolith is visible at the edge of frame.** |
| **A2-S03** | 25–30 | 1:11.429 | 5 | stretch (3–32) | 1 | **The bulla breaks**, c. 3300 BCE. It splits along a real seam and tokens — 1×1 round plates, Terrakotta — spill out and arrange themselves into a counting row, one per beat. **Then the count overruns**: tokens keep spilling past what the row can hold and past what the eye can track. Caption: **"An account you cannot alter without breaking it."** *No numeral, no HUD counter.* Tallying by object ceasing to scale is the actual historical reason for the bulla, and it is an image rather than a caption on a caption. **The overrun is built as its own scene** (`tokensUeberlauf`, 200 counters against the row's 40, its own dissolve from t=0.55): one scene of 240 would arrive as a single spreading stain, because a dissolve writes one amount to every instance and the erosion noise is sampled in world position. Measured: 21 separable counters at t=0.5, 61 at t=0.7, and at t=0.95 twice the pixels in fewer blobs — they stop being objects and become a heap. |
| **A2-S04** | 30–34 | 1:25.714 | 4 | stretch (2–25) | 1 | **Sardis, mid-6th century BCE.** The tokens compress into a single cylinder — 2×2 round bricks + a tile, Elektrum, real metallic finish. A die descends and **strikes on the beat**: one hard accent, one bloom flash, one frame of white. |
| **A2-S05** | 34–37 | 1:37.143 | 3 | stretch (2–21) | 1 | **The struck face.** Macro, rotating, chrome reflections from the procedural environment. Caption: **"Not the first coin. The first coin you did not have to test."** Early electrum's gold-to-silver ratio varied and could be manipulated; fungibility arrives with Kroisos's bimetallic reform — a *better* fact for this thesis than the one rev 1 had. The fugue reaches its first full cadence here. |

---

### ACT III — DIE FUGE · bars 37–57 · 1:45.714–2:42.857

*Mass production, and the line of ownership. The joint becomes an industry.*

| Shot | Bars | In | Dur | Scaling | Tier | Content |
|---|---|---|---|---|---|---|
| **A3-S01** | 37–41 | 1:45.714 | 4 | stretch (2–21) | 1 | **Two options, decided at authoring time — rev 1 conflated them.** Either *"Pont du Gard, c. 50 CE"* — an `Arch` over a `Colonnade` in Steingrau, keystone dropping last, the structure settling by one plate as it takes the load, captioned **"the module before the mortar"** (the Pont du Gard is dry-laid limestone ashlar and contains no brick at all) — or *"Rome, c. 110 CE — the bipedalis bonding course, and the brick stamp that dates it by consul."* **The second is the better image for this thesis:** a standard module with an audit trail. |
| **A3-S02** | 41–45 | 1:57.143 | 4 | stretch (2–21) | 1 | **Hard cut to Patentweiß.** A white studio, no horizon. One brick rotates at centre: the **Batima** system. Patent-drawing overlay derived from the real published drawing, credited. Lower third: **`Le Batima Soc. · FR 588985, angemeldet 1924 — der Stein ist älter`** — *not* `BE 311029 · 1923`, which could not be confirmed, and no longer `c. 1924` either. Google Patents gives FR588985A, "Jeu de construction", Le Batima Soc.: **priority 6 June 1923, filed 5 June 1924, published 18 May 1925**, classified A63H33/088, *building blocks with holes*. Nothing was granted in 1924. And the object is older than its patent — architoys.net dates the toy 1900–10, a PBA Galleries lot c. 1905, a collectors' thread names a series running 1900–1914 beside the 1924–1949 boxes — so the caption says that rather than implying the patent is the origin. This is the one genuinely great cut in the piece; protect it by making it the only shock of its magnitude. |
| **A3-S03** | 45–48 | 2:08.571 | 3 | stretch (2–16) | 1 | **Kiddicraft.** A second brick materialises beside the first, same lighting, same framing; the two align; a cutaway reveals the anti-stud. Lower third: **`GB 529580 · Hilary Fisher Page, eingereicht 17. April 1939`**. Rev 3 said "filed 17 April 1940 — not 1939" and had it exactly backwards: Google Patents gives GB529580A, "Improvements in toy building blocks", H. F. Page, **priority and filing both 17 April 1939, publication 25 November 1940** — the ordinary shape of a GB specification of that era, applied for in one year and accepted in the next. The patent's own abstract is also the best evidence in the piece for why these bricks are tubeless: a hollow cube with "four circular bosses on its upper face, so that the walls of a superimposed block will contact the bosses tangentially so preventing lateral movement". Shorter than A3-S02 on purpose: it is a rhyme, and a rhyme must be shorter than what it rhymes with. **And the brick is now Page's brick rather than a modern one with a hole in it:** the Brighton Toy and Model Index says the Kiddicraft bricks "have mildly rounded vertical edges and bobbled tops to make it easier to locate the bricks with their neighbours", against a modern Lego brick's "flat tops (with the Lego logo), and sharp vertical edges". Both are a function, not a finish — the brick is shaped to be found by the next brick — and a macro that leaves them out is showing the wrong object. |
| **A3-S04** | 48–53 | 2:17.143 | **5** | stretch (3–32) | 1 | **1949, and the clutch. The title image of the work, and now the longest shot in its act.** A third brick joins; the three snap into one column. **Macro:** a stud entering a tube, real LDraw geometry, filling the frame. *Implementation note:* LDraw's stud OD and tube ID are both nominally 12 LDU and therefore interpenetrate exactly — an explicit clearance must be authored for this shot and labelled as authored. Lower third: `Automatic Binding Bricks · 1949`. **The monolith is visible at the edge of frame.** **A3-S03 and A3-S04 are no longer one mesh under two captions.** The same index, on the 1949 brick: "Like the Page bricks, the first Lego bricks had slotted ends with a tapered slot, and their dimensions were pretty much indistinguishable from Page's — they were quite clearly copies" — and Lego dropped the rounded edges and flattened the studs, which years later became the space for the logo. So the copy kept everything that was about the *system* and left behind the two things that were about the *hand*. Showing that difference is a better way of showing the sameness than asserting it with one file. |
| **A3-S05** | 53–57 | 2:31.429 | 4 | stretch (2–21) | 1 | **Multiplication.** The column becomes a grid becomes a field, thousands of instanced bricks marching outward. **The stretto begins** — subject entries overlap at half their own length, so the visual multiplication and the contrapuntal multiplication are the same event. Lower third, held 3 s: `Interlego AG v Tyco Industries Inc · [1988] UKPC 3`, captioned **"the case that ended the drawings, not the case that named the inventor."** Lego lost it, and it adjudicates nothing about Fisher Page. |

**The Fisher Page text**, wherever the piece states its claim (credits, the
Postillen, the chronicle): the bounded version is strong and defensible, the
absolute version is false.

> Hilary Fisher Page patented and sold the self-locking studded brick before
> Billund made one. LEGO's own history says so. What Billund added in 1958 —
> the tube inside the brick — is real, and it is what made the joint hold.
> Two inventions, one object. Only one of them is remembered.

---

### ATLAS — DER ATLAS DER FUGE · bars 57–63 · 2:42.857–3:00.000

*The module, applied to the world. The scalable movement — this is where the longer cuts live.*

The Atlas is a repeating unit of **2 bars**, authored once and instantiated
N times. In the canonical cut: **3 sites**.

| Sub-beat | Share | Content |
|---|---|---|
| **ATL-a** | ~⅓ bar | The world plate: a low, dark field of 1×1 plates. The camera arrives at the site's real coordinates (equirectangular, stated as such). |
| **ATL-b** | ~¾ bar | **The site builds itself** from its recipe, bottom-up, staggered by build step, in its own real palette. |
| **ATL-c** | ~¼ bar | **The flag appears** — a flat brick mosaic, *no pole and no wind by default*, together with its `QuantizeReport`: the ΔE, and the colours LDraw does not have. A flag that cannot be built in the available palette is this thesis's best counter-evidence, and showing it costs nothing. |
| **ATL-d** | — | **One chronicle card per movement**, not per site. It states the *module* — which bond, which unit of measure — because that is the only fact belonging to this argument. Per-site name and year go to a single quiet corner line. |
| **ATL-e** | remainder | The camera arcs once around the pair. |

**The Atlas accumulates.** Sites stay on the world plate; the movement builds
a populated globe rather than resetting to empty. What is withheld varies —
sometimes the flag before the build, sometimes the card only after the site
is gone; every fifth site drops to macro on one joint; each site binds to a
different fugue voice, so four sites are one entry cycle and the music tells
you where you are. Without this, site 40 looks exactly like site 1 and the
movement is a slideshow with a good render.

| Cut | Atlas | Sites | Unit |
|---|---|---|---|
| 4:00 | 6 bars | 3 | 2 bars |
| 10:00 | ~2:24 | 12 | ~4 bars |
| 60:00 | ~30:00 | 40 | ~16 bars |
| endless | 6 bars/cycle | 3, rotating with the cycle seed | 2 bars |

Site selection for the canonical cut is **the three the thesis needs — a
brick site, a stone-arch site, and one non-European** — not the three the
correspondence needs. "The Postilla states" reads on screen as arbitrary
curation.

---

### ACT IV — DER TOKEN · bars 63–84 · 3:00.000–4:00.000

*The module stops being physical, and does not stop being a module.*

| Shot | Bars | In | Dur | Scaling | Tier | Content |
|---|---|---|---|---|---|---|
| **A4-S01** | 63–68 | 3:00.000 | 5 | stretch (3–42) | 1 | **Der Inkpour.** Everything on screen — sites, flags, the monolith held in the background all along — dissolves into points, each brick becoming its own swarm drifting outward along its own normals. Terrakotta and Steingrau drain from the palette. The pulse enters underneath, half-time. |
| **A4-S01b** | 68–70 | 3:14.286 | **2** | fixed | 1 | **DER LETZTE STEIN.** One brick. Terrakotta. Alone in black, held still, no camera move, while the colour drains out of it and it goes to point-swarm **last of everything on screen**. The network arrives *after* it is gone, not as its continuation. This is the shot rev 1 did not have, and it is what converts the argument into an arc: the piece stops saying "the module became a token" — monotonic, costless, unfeelable — and starts saying "the thing we could hold became a thing we can't." |
| **A4-S02** | 70–76 | 3:20.000 | 6 | stretch (4–50) | 1 | **The field forms.** The points reorganise into a regular lattice, Terminalgrün, activations propagating. **Rendered as an inventory, not as science fiction** — a parts bin, a bill of materials, counted. More accurate about what a model is, and far less exhausted as an image. The shot's whole job is to make "neural network" and "stud grid" the same picture. |
| **A4-S03** | 76–80 | 3:37.143 | 4 | stretch (2–50) | 1 | **Tokens.** Each node emits a glyph that travels an edge and is absorbed. The chronicle line runs beneath, ending at **2017 — "the year the module became the only unit."** *No paper title on screen*: it dates the day it renders, and subword tokenisation predates that paper anyway. The pulse doubles. The fugue's subject is now in the bass, quantised to sixteenths — same intervals, different century. |
| **A4-S04** | 80–83½ | 3:48.571 | 3½ | stretch (2–20) | 1 | **Saturation.** The lattice fills the frame and keeps growing past it; bloom rises; the camera stops moving. Everything is Terminalgrün on black. A pedal point in the bass under the full four voices. |
| **DER KICK** | 83½–84 | 3:58.571 | **½ (2 beats)** | **fixed** | 1 | On the final accent — **one event, both meanings of the word** — the camera zooms out exponentially by 10⁴. The network collapses toward the centre; bloom collapses with it. At 4:00.000 exactly, what remains is **one Terminalgrün pixel at frame centre** on black: the pixel A1-S01 opened on. Audio cuts to the same 55 Hz sine, and the loop closes. Rev 1's 800 ms was neither one beat nor two, so it could not be frame-exact against its own accent — and at 10⁴ zoom, 800 ms is a flash where the collapse should be *seen* collapsing. |

---

## 5. The captions, corrected

Every dated claim in rev 1 was fact-checked. These are the corrections that
appear on screen; the reasoning and sources are in
[`../FUGEN-ENGINE-REVIEW-01.md`](../FUGEN-ENGINE-REVIEW-01.md) §4.

| Rev 1 | Rev 3 |
|---|---|
| "Mesopotamia ~3500 BCE — first standardised mud bricks" | **"Uruk, ~3500 BCE — the brick is sized to the hand that carries it."** |
| "This is the invention of number" | **"An account you cannot alter without breaking it."** (c. 3300 BCE) |
| "Lydia ~600 BCE — the first fungible unit" | **"Sardis, mid-6th c. BCE. Not the first coin. The first coin you did not have to test."** |
| "Rome ~100 CE — bipedalis" over the Pont du Gard | Either **"Pont du Gard, c. 50 CE — the module before the mortar"** or **"Rome, c. 110 CE — the bipedalis bonding course, and the brick stamp that dates it by consul."** |
| "Stonehenge ~3000 BCE" | **c. 2500 BCE** for the sarsens and their mortice-and-tenon joints. |
| `BE 311029 · 1923` | ~~`FR 588985 · Le Batima Soc. · c. 1924`~~ → **`Le Batima Soc. · FR 588985, angemeldet 1924 — der Stein ist älter`** (priority 1923-06-06, filed 1924-06-05, published 1925-05-18) |
| `GB 529580 · 1939` | ~~`GB 529580 · filed 17 April 1940`~~ → **`GB 529580 · Hilary Fisher Page, eingereicht 17. April 1939`** (rev 3 corrected the right date to a wrong one; 1940-11-25 is the publication) |
| "Interlego v Tyco — the confirmation" | **"the case that ended the drawings, not the case that named the inventor."** |
| "2017 · Attention Is All You Need" | **"2017 — the year the module became the only unit."** |
| "Dies ist keine Metapher. Dies ist Mathematik." | **"Dies ist eine Metapher. So bauen Menschen."** The AI token is a pun on the clay token, not a descent from it — and the formal argument (discrete, interchangeable, positionally addressed units under a combinatorial grammar) survives being called what it is. |

**Three additions that strengthen the thesis rather than decorate it**, for
the Atlas and for the longer cuts:

- **Indus Valley, c. 2600–1900 BCE** (Mohenjo-daro, already a World Heritage
  Site): fired brick at a consistent **1 : 2 : 4** ratio across a territory
  larger than Sumer, plus a binary/decimal weight system. A better
  standardisation case than Mesopotamia — and it **rhymes with the
  monolith's own 1 : 4 : 9**.
- **Qin China, 3rd c. BCE**: crossbow triggers and terracotta-army components
  made as interchangeable parts with workshop maker's marks. Interchangeable
  manufacture *plus* stamped accountability, two millennia before Europe.
- **Yingzao Fashi, 1103 CE (Li Jie)**: the *cai-fen* system — eight graded
  timber sections from which every member derives by ratio. The
  best-documented pre-modern parametric building standard anywhere, and it is
  a *ratio* system, which is what this work actually is.

## 6. The four cuts, and what the loop remembers

| Cut | `--duration` | Tiers | Atlas | Purpose |
|---|---|---|---|---|
| **Der Schnitt** | `240` | 1 | 3 sites | The canonical work. **The deliverable.** |
| **Die Fassung** | `600` | 1 + 2 | 12 sites | Gallery loop, lecture. Every act breathes; the Atlas becomes a movement. |
| **Die Installation** | `3600` | 1 + 2 + 3 | 40 sites | Museum hour. **Post-date** — see [`README.md`](README.md)'s scope decision. |
| **Die Schleife** | `endless` | 1 | 3, rotating | Permanent installation. Cycle *n* uses `splitmix64(seed ^ n)`. |

**The loop seam is a hard requirement:** the final frame of any cut and the
first frame of any cut are **pixel-identical** — black field, one Terminalgrün
pixel at centre — and the audio arrives at and departs from the same 55 Hz
sine at the same phase. The endless cut crossfades nothing; it continues,
because there is nothing to crossfade.

*Implementation note:* the final pixel must be drawn as a fixed-radius disc
in a pass that **bypasses the composer**. A one-device-pixel dot rendered
through SMAA and bloom is DPR-dependent, and pixel-identity would silently
fail on a different display.

**What the loop is allowed to remember.** The creative-direction review argued
for breaking the loop outright — one brick that does not return per cycle —
on the grounds that a perfectly closed loop asserts "nothing is lost", which
is the truism rendered structurally. That is probably the better artwork, and
it is rejected only as a default, because pixel-identity is load-bearing for
the endless installation and for the on-chain edition's determinism. **The
compromise, adopted:** the seam stays identical, and in endless mode the
world plate's accumulated dark region **persists across cycles**. The pixel is
the pixel; by cycle forty the field around it is not empty.

## 7. The hidden inscriptions

1. **`GB 587,206`** — Fisher Page's real patent — in the instance metadata of
   every brick-shaped object in the work. *Verify the record on Espacenet
   before printing this number on anything physical.*
2. **`IA-2026-002`** — the archive signature, in the world plate's own tile
   pattern, readable only from directly above.
3. **The seed** — in the credits, so any frame anyone screenshots traces back
   to the run that produced it.

*A note on A2-S05's struck inscription:* no lettering geometry exists in
LDraw, and a texture would break Rule 1. Either build the inscription from
1×1 plates at mosaic scale (legible only in macro, which is the shot anyway)
or drop it from the coin and keep it in metadata. Decide before M79.

---

*Iunctura Archiv · Signatur IA-2026-002 · screenplay rev 3 · 2026-07-25*
*Zählen. Bauen. Berechnen. Derselbe Instinkt. Für immer.*
