"""Emits the two pre-1958 bricks of Act III, into `ldraw-scenes/parts/`.

    python3 scripts/ldraw/gen-prewar-bricks.py     # run from the repo's spex/

`kiddicraft-2x2.dat` is Hilary Page's Self-Locking Building Brick and
`abb-2x2.dat` is Lego's 1949 Automatic Binding Brick. They come out of one
function because the sources say they were one object apart from two features,
and a generator is the only way to keep "apart from two features" true: change
the slot here and both bricks change, which is what the history says happened.

WHY THIS IS A SCRIPT AND NOT TWO HAND-WRITTEN FILES
---------------------------------------------------
Because a hand-written one was wrong for an afternoon in a way that looked like
broken geometry and was not.

EVERY FACE'S WINDING HAS TO POINT OUTWARD. `viewer/src/mesh/materials.ts` binds
opaque materials as `THREE.FrontSide`, so a quad whose vertex order gives an
inward normal is not a dark face - it is NOT DRAWN, and what you see through the
hole is the inside of the far wall. The first version of this brick was written
by hand with the vertex orders copied from wherever they read most naturally,
and it rendered as a brick with its top missing and its walls floating: the
triangle count and the bounding box were both exactly right, which is why an
hour went into looking for missing geometry that was all there.

So `orient()` takes the outward normal as an argument at every call site and
fixes the order itself. `cross(b - a, c - a)` must point the way the face
faces - measured against the official `box5.dat`, whose side faces do exactly
that, and confirmed by rendering.

(The part this replaced had inconsistent windings too, and got away with it
because the faces it happened to lose were ones no camera in the piece looks at.)
"""
import os
INWARD = os.environ.get("INWARD") == "1"  # the other convention, for bisecting
H, O, I = 24.0, 20.0, 16.0
R = 2.0
SLOT_OPEN, SLOT_TIP, SLOT_TOP = 3.0, 2.0, 11.0

UP = (0, -1, 0)
DOWN = (0, 1, 0)

def build(rounded, stud_part):
    F = O - R if rounded else O
    L = []
    FLIP = -1.0 if INWARD else 1.0
    def cross(a, b, c):
        u = [b[i] - a[i] for i in range(3)]
        v = [c[i] - a[i] for i in range(3)]
        return [u[1]*v[2]-u[2]*v[1], u[2]*v[0]-u[0]*v[2], u[0]*v[1]-u[1]*v[0]]
    def orient(pts, n):
        c = cross(pts[0], pts[1], pts[2])
        if sum(c[i]*n[i] for i in range(3)) * FLIP < 0:
            return list(reversed(pts))
        return pts
    def q(*p, n=None, col=16):
        p = orient(list(p), n) if n else list(p)
        L.append(f"4 {col} " + " ".join(f"{v:g}" for a in p for v in a))
    def tri(*p, n=None, col=16):
        p = orient(list(p), n) if n else list(p)
        L.append(f"3 {col} " + " ".join(f"{v:g}" for a in p for v in a))
    def line(a, b):
        L.append("2 24 " + " ".join(f"{v:g}" for p in (a, b) for v in p))

    L.append("0 BFC INVERTNEXT")
    L.append("1 16 0 24 0 16 0 0 0 -20 0 0 0 16 box5.dat")
    L.append("")
    L.append("0 // top face (y=0)")
    q((-F,0,F), (F,0,F), (F,0,-F), (-F,0,-F), n=UP)
    if rounded:
        q((-F,0,-F), (F,0,-F), (F,0,-O), (-F,0,-O), n=UP)
        q((-F,0,O), (F,0,O), (F,0,F), (-F,0,F), n=UP)
        q((-O,0,F), (-F,0,F), (-F,0,-F), (-O,0,-F), n=UP)
        q((F,0,F), (O,0,F), (O,0,-F), (F,0,-F), n=UP)
    L.append("")
    if rounded:
        L.append("0 // the four rounded vertical edges, r = 2 LDU")
        for sx, sz in ((1,1), (1,-1), (-1,1), (-1,-1)):
            x, z = sx*F, sz*F
            L.append(f"1 16 {x:g} 0 {z:g} {sx*R:g} 0 0 0 {H:g} 0 0 0 {sz*R:g} 1-4cyli.dat")
            L.append(f"1 16 {x:g} 0 {z:g} {sx*R:g} 0 0 0 1 0 0 0 {sz*R:g} 1-4disc.dat")
            L.append(f"1 16 {x:g} 0 {z:g} {sx*R:g} 0 0 0 1 0 0 0 {sz*R:g} 1-4edge.dat")
            L.append(f"1 16 {x:g} {H:g} {z:g} {sx*R:g} 0 0 0 1 0 0 0 {sz*R:g} 1-4edge.dat")
        L.append("")
    else:
        L.append("0 // sharp vertical edges: four hard lines, and no fillet at all")
        for sx, sz in ((1,1), (1,-1), (-1,1), (-1,-1)):
            line((sx*O, 0, sz*O), (sx*O, H, sz*O))
        L.append("")

    L.append("0 // the four walls, each with its tapered end slot: 6 LDU across at the")
    L.append("0 // open end, 4 at the top of the cut, 13 deep.")
    for s in (-1, 1):
        # wall at z = s*O, running in x. `u` is the in-plane axis.
        def P(u, y):
            return (u, y, s*O)
        q(P(-F,H), P(-SLOT_OPEN,H), P(-SLOT_OPEN,0), P(-F,0), n=(0,0,s))
        q(P(SLOT_OPEN,H), P(F,H), P(F,0), P(SLOT_OPEN,0), n=(0,0,s))
        q(P(-SLOT_OPEN,SLOT_TOP), P(SLOT_OPEN,SLOT_TOP), P(SLOT_OPEN,0), P(-SLOT_OPEN,0), n=(0,0,s))
        tri(P(-SLOT_OPEN,SLOT_TOP), P(-SLOT_OPEN,H), P(-SLOT_TIP,SLOT_TOP), n=(0,0,s))
        tri(P(SLOT_OPEN,SLOT_TOP), P(SLOT_TIP,SLOT_TOP), P(SLOT_OPEN,H), n=(0,0,s))
        line(P(-F,0), P(F,0))
        line(P(-F,H), P(F,H))
        def Q(u, y):
            return (s*O, y, u)
        q(Q(-F,H), Q(-SLOT_OPEN,H), Q(-SLOT_OPEN,0), Q(-F,0), n=(s,0,0))
        q(Q(SLOT_OPEN,H), Q(F,H), Q(F,0), Q(SLOT_OPEN,0), n=(s,0,0))
        q(Q(-SLOT_OPEN,SLOT_TOP), Q(SLOT_OPEN,SLOT_TOP), Q(SLOT_OPEN,0), Q(-SLOT_OPEN,0), n=(s,0,0))
        tri(Q(-SLOT_OPEN,SLOT_TOP), Q(-SLOT_OPEN,H), Q(-SLOT_TIP,SLOT_TOP), n=(s,0,0))
        tri(Q(SLOT_OPEN,SLOT_TOP), Q(SLOT_TIP,SLOT_TOP), Q(SLOT_OPEN,H), n=(s,0,0))
        line(Q(-F,0), Q(F,0))
        line(Q(-F,H), Q(F,H))
    L.append("")
    L.append("0 // bottom rim (y=24), tiled to match the top minus the cavity")
    if rounded:
        q((-F,H,-F), (F,H,-F), (F,H,-O), (-F,H,-O), n=DOWN)
        q((-F,H,O), (F,H,O), (F,H,F), (-F,H,F), n=DOWN)
        q((-O,H,F), (-F,H,F), (-F,H,-F), (-O,H,-F), n=DOWN)
        q((F,H,F), (O,H,F), (O,H,-F), (F,H,-F), n=DOWN)
    q((-F,H,-I), (F,H,-I), (F,H,-F), (-F,H,-F), n=DOWN)
    q((-F,H,F), (F,H,F), (F,H,I), (-F,H,I), n=DOWN)
    q((-F,H,I), (-I,H,I), (-I,H,-I), (-F,H,-I), n=DOWN)
    q((I,H,I), (F,H,I), (F,H,-I), (I,H,-I), n=DOWN)
    L.append("")
    L.append("0 // the studs")
    for sx in (-10, 10):
        for sz in (-10, 10):
            L.append(f"1 16 {sx} 0 {sz} 1 0 0 0 1 0 0 0 1 {stud_part}")
    return "\n".join(L) + "\n"

SHARED = """0 //
0 // BOTH BRICKS ARE HOLLOW AND HAVE NO TUBE. The tube inside a brick is the
0 // 1958 patent; a Self-Locking brick and an Automatic Binding Brick grip by
0 // their studs against the walls alone, which is why they held so badly.
0 // `ldraw-scenes/klemme.ldr` is the real 3003 with its tubes, and that shot
0 // is the difference.
0 //
0 // Brick 2 x 2: 40 x 40 x 24 LDU, origin at the footprint centre and at the
0 // TOP of the part, LDraw +Y down - a brick on the ground plane is translated
0 // to y = -24. No `BFC CERTIFY`: the walls are single-sided quads and the
0 // renderer draws them from both faces.
0 //
0 // Generated geometry, not a flattened 3003.dat. The previous version of this
0 // file WAS a flattened 3003 with the one interior tube line deleted, which
0 // made both pre-1958 bricks a modern LEGO brick with a hole in it - the exact
0 // claim the act spends three shots arguing against.
"""

KIDDI = """0 ~Self-Locking Building Brick 2 x 2, Kiddicraft - Hilary Page's brick
0 Name: kiddicraft-2x2.dat
0 Author: spex project (hand-modelled; see below)
0 !LDRAW_ORG Unofficial_Part
0 !LICENSE Licensed under CC BY 4.0 : see CAreadme.txt
0 // The Brighton Toy and Model Index says it in one sentence: "The Kiddicraft
0 // bricks have mildly rounded vertical edges and bobbled tops to make it
0 // easier to locate the bricks with their neighbours."
0 //
0 // Both halves of that are a FUNCTION and not a finish: Page's brick is shaped
0 // to be found by the next brick. A3-S03 is a macro of this object in an act
0 // about who invented what, so the two features that tell it apart by eye are
0 // the two it cannot be missing.
0 //
0 // ROUNDED EDGES: fillet radius 2 LDU = 0.8 mm on a 16 mm brick, which is what
0 // "mildly" buys - a soft highlight down each corner, not a visible bevel.
0 // Four `1-4cyli` quarter cylinders at (+-18, +-18), the walls inset to +-18
0 // where the arcs take over, both faces tiled to match with `1-4disc` corners.
0 // THERE ARE NO VERTICAL EDGE LINES, and that is the point: the arcs are
0 // tangent to the walls, so the corner is drawn by the conditional lines
0 // `1-4cyli` carries and by nothing else.
0 //
0 // BOBBLED TOPS: `kiddicraft-stud.dat`, beside this file.
0 //
0 // TAPERED SLOT: "the first Lego bricks had slotted ends with a tapered slot,
0 // and their dimensions were pretty much indistinguishable from Page's" - so
0 // the taper is Page's too. 6 LDU across at the open end, 4 at the top of the
0 // cut, 13 deep. It takes the guillotine windows and the card inserts, which
0 // is what the slots are FOR.
""" + SHARED + "\n"

ABB = """0 ~Automatic Binding Brick 2 x 2, 1949 - the copy, with two things missing
0 Name: abb-2x2.dat
0 Author: spex project (hand-modelled; see below)
0 !LDRAW_ORG Unofficial_Part
0 !LICENSE Licensed under CC BY 4.0 : see CAreadme.txt
0 // THIS FILE EXISTS BECAUSE THE COPY WAS NOT IDENTICAL, and finding that out
0 // made the act better rather than worse.
0 //
0 // The Brighton Toy and Model Index on the 1949 brick: "Like the Page bricks,
0 // the first Lego bricks had slotted ends with a tapered slot, and their
0 // dimensions were pretty much indistinguishable from Page's - they were quite
0 // clearly copies." And then, in the same breath: Lego dropped Page's rounded
0 // vertical edges and flattened the stud tops, which "some years later" became
0 // the space for the logo.
0 //
0 // So the two features the copy left behind are exactly the two that were
0 // about the HAND - the rounded edge and the bobbled stud are what make a
0 // brick findable by feel - and everything that was about the SYSTEM was kept
0 // to the millimetre. This brick is `kiddicraft-2x2.dat` with sharp corners
0 // and `stud.dat`, and it is identical in every other number, because that is
0 // what the sentence says.
0 //
0 // Until now A3-S03 and A3-S04 showed one part twice under two captions, on
0 // the argument that they ARE the same object. They are the same object in
0 // the sense Interlego v Tyco is about, and they are not the same shape, and
0 // showing the second is a better way of showing the first.
""" + SHARED + "\n"

open('ldraw-scenes/parts/kiddicraft-2x2.dat','w').write(KIDDI + build(True, 'kiddicraft-stud.dat'))
open('ldraw-scenes/parts/abb-2x2.dat','w').write(ABB + build(False, 'stud.dat'))
print("ok")
