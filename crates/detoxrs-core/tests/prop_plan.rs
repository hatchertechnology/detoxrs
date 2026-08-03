//! §8.2's property tests against `plan()`, minus Undo round-trip.
//!
//! Six of §8.2's seven properties are here. **Undo round-trip is deliberately
//! absent**: it is quantified over "apply the plan then replay the journal", and
//! neither `apply` nor the journal exists yet (they are M1 work package 5b). It
//! cannot be written against `plan()` alone, and a stub asserting nothing would be
//! worse than its absence.
//!
//! Two scoping rules, both taken from the property statements themselves:
//! every property about destinations is quantified over the `Rename` items only
//! (an `Unchanged`/`Skipped`/`Conflict` item's `to` is its own `from`, so it
//! renames nothing and cannot clobber anything), and the comparison key is
//! recomputed here from the property's own words -- NFC, case-folded only on a
//! volume declared case-insensitive -- rather than borrowed from the
//! implementation, which would make every property vacuous.

mod support;

use detoxrs_core::plan::{
    DirIdent, Entry, EntryKind, Ident, OnCollision, Plan, PlanError, PlanItem, Resolution,
    VolumeCase, plan,
};
use detoxrs_core::policy::Policy;
use proptest::prelude::*;
use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use unicode_normalization::UnicodeNormalization as _;

// ---- the property's own comparison key, written independently -----------------

/// §6.2: always NFC; case-folded **only** when the volume says so.
fn key(name: &OsStr, case: VolumeCase) -> Vec<u8> {
    name.to_str().map_or_else(
        || {
            // Opaque names are never renamed, but they do occupy a name, so they
            // need a key that cannot alias a UTF-8 one. 0xFF is not a UTF-8 byte.
            let mut k = vec![0xFF];
            k.extend_from_slice(name.as_encoded_bytes());
            k
        },
        |s| {
            let nfc: String = s.nfc().collect();
            match case {
                VolumeCase::Sensitive => nfc.into_bytes(),
                VolumeCase::Insensitive => {
                    nfc.to_lowercase().nfc().collect::<String>().into_bytes()
                }
            }
        },
    )
}

fn renames(p: &Plan) -> impl Iterator<Item = &PlanItem> {
    p.items
        .iter()
        .filter(|i| i.resolution == Resolution::Rename)
}

/// `dir/name`, the full path an item refers to.
fn path_of(dir: &Path, name: &OsStr) -> PathBuf {
    dir.join(name)
}

// ---- generators ---------------------------------------------------------------

/// Names chosen to make collisions and near-swaps frequent rather than
/// astronomically unlikely.
///
/// The near-swap pool is §8.2's explicit requirement for the No-sibling-chains
/// property: `a_b`/`a-b`/`a b`, `A.txt`/`a.txt`, and an NFC/NFD pair, which the
/// Order-safety property does not exercise at all. Every pool entry is a name a
/// real filesystem would accept, and several of them are already fixed points of
/// `transform`, which is what turns the *other* member of the pair into a
/// pre-existing-destination conflict instead of a chain.
const NEAR_SWAPS: &[&str] = &[
    "a b",
    "a_b",
    "a-b",
    "a  b",
    "a b.txt",
    "a_b.txt",
    "a  b.txt",
    "A.txt",
    "a.txt",
    "caf\u{e9}.txt",
    "cafe\u{301}.txt",
    "caf\u{e9} .txt",
    "IMG 0042.JPG",
    "IMG_0042.JPG",
    "IMG-0042.JPG",
    "x",
    "x-2",
    "x 2",
    "x-2.txt",
    ".hidden file",
    ".hidden_file",
    "***",
    "-",
    "..",
    "report.tar.gz",
    "report .tar.gz",
];

fn plan_name() -> impl Strategy<Value = String> {
    prop_oneof![
        4 => proptest::sample::select(NEAR_SWAPS).prop_map(str::to_owned),
        1 => support::nasty_name(),
    ]
    // A directory entry's name can contain neither `/` nor NUL on any supported
    // platform, so generating them would quantify over snapshots that cannot
    // exist -- and a `/` inside a name would make "is this path inside that
    // directory?" ambiguous in Order safety's own statement.
    .prop_map(|s| s.replace(['/', '\0'], "~"))
    .prop_filter("a directory entry always has a name", |s| !s.is_empty())
}

fn volume_case() -> impl Strategy<Value = VolumeCase> {
    prop_oneof![Just(VolumeCase::Sensitive), Just(VolumeCase::Insensitive)]
}

fn on_collision() -> impl Strategy<Value = OnCollision> {
    prop_oneof![
        Just(OnCollision::Number),
        Just(OnCollision::Skip),
        Just(OnCollision::Fail),
    ]
}

const fn ident(ino: u64) -> Ident {
    Ident {
        dev: 1,
        ino,
        nlink: 1,
        mtime: SystemTime::UNIX_EPOCH,
    }
}

/// A directory identity for a fixture that has no real filesystem behind it:
/// deterministic per `dir`, so two entries built with the same `dir` compare
/// equal and two different `dir`s (almost certainly) do not -- the same
/// contract `walk.rs`'s real `dir_ident_of` gives `plan()`, without needing an
/// actual filesystem to probe.
fn dir_ident(dir: &Path) -> DirIdent {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    dir.hash(&mut h);
    (1, h.finish())
}

/// A three-level snapshot whose two intermediate directories have names that
/// `transform` will change, so Order safety is exercised rather than trivially
/// satisfied by clean directory names.
fn snapshot() -> impl Strategy<Value = Vec<Entry>> {
    (
        proptest::collection::vec(plan_name(), 0..4),
        proptest::collection::vec(plan_name(), 0..4),
        proptest::collection::vec(plan_name(), 0..3),
    )
        .prop_map(|(root, mid, deep)| build_snapshot(&root, &mid, &deep))
}

fn build_snapshot(root: &[String], mid: &[String], deep: &[String]) -> Vec<Entry> {
    let root_dir = PathBuf::from("t");
    let mid_dir = root_dir.join("a b");
    let deep_dir = mid_dir.join("c d");

    let mut out = Vec::new();
    let mut seen: HashSet<(PathBuf, OsString)> = HashSet::new();
    let mut ino = 0u64;

    let mut push = |dir: &Path, name: &str, kind: EntryKind, out: &mut Vec<Entry>| {
        let os = OsString::from(name);
        if !seen.insert((dir.to_path_buf(), os.clone())) {
            return; // a real snapshot never lists the same entry twice
        }
        ino += 1;
        out.push(Entry {
            dir: dir.to_path_buf(),
            name: os,
            kind,
            ident: ident(ino),
            dir_ident: dir_ident(dir),
            depth: u32::try_from(dir.components().count()).unwrap_or(u32::MAX),
        });
    };

    push(&root_dir, "a b", EntryKind::Dir, &mut out);
    push(&mid_dir, "c d", EntryKind::Dir, &mut out);
    for n in root {
        push(&root_dir, n, EntryKind::File, &mut out);
    }
    for n in mid {
        push(&mid_dir, n, EntryKind::File, &mut out);
    }
    for n in deep {
        push(&deep_dir, n, EntryKind::File, &mut out);
    }
    out
}

/// `plan()` must not error on any snapshot a walk could produce: `BatchRefused`
/// is the one legitimate error and only under `fail`, and
/// `InternalInconsistency` firing at all is §5.3's Idempotence proof breaking.
fn planned(entries: &[Entry], p: &Policy, oc: OnCollision, case: VolumeCase) -> Option<Plan> {
    match plan(entries, p, oc, case) {
        Ok(plan) => Some(plan),
        Err(PlanError::BatchRefused(items)) => {
            assert_eq!(oc, OnCollision::Fail, "batch refused under {oc:?}");
            assert!(!items.is_empty(), "refused with no conflicting item");
            None
        }
        Err(PlanError::InternalInconsistency(msg)) => {
            panic!("sibling-chain assertion fired -- Idempotence broke first: {msg}")
        }
    }
}

proptest! {
    // Fixed seed (§5's finding): unseeded, a mutation only some generated
    // inputs expose gets a non-deterministic verdict -- the review measured
    // one killed 7 times in 12 runs. A fixed seed makes a green run mean the
    // same thing on the next run, which is what a mutation-tested module
    // needs; the case count stays at proptest's own default (256) rather than
    // growing, since determinism, not volume, was the gap.
    #![proptest_config(ProptestConfig {
        cases: 256,
        rng_seed: proptest::test_runner::RngSeed::Fixed(0x6465_746f_7872_7331),
        ..ProptestConfig::default()
    })]

    /// **No collision.** The `Rename` items have pairwise-distinct
    /// `(dir, NFC(casefold?(to)))`. The executable form of the maintainer's #130
    /// objection: this is the check that stops N files collapsing into 1.
    #[test]
    fn no_collision(
        entries in snapshot(),
        p in support::policy_or_default(),
        oc in on_collision(),
        case in volume_case(),
    ) {
        let Some(plan) = planned(&entries, &p, oc, case) else { return Ok(()) };
        let mut seen: HashSet<(PathBuf, Vec<u8>)> = HashSet::new();
        for item in renames(&plan) {
            let k = (item.dir.clone(), key(&item.to, case));
            prop_assert!(
                seen.insert(k),
                "two renames share a destination: {:?} -> {:?}", item.from, item.to
            );
        }
    }

    /// **No pre-existing clobber.** No `Rename` item's `to` equals a snapshot
    /// entry that is not that item's own `from`.
    ///
    /// Plan-time half only, by construction: `plan()` has no I/O, so the
    /// apply-time recheck and the kernel's `RENAME_NOREPLACE` refusal are the
    /// §8.4 TOCTOU row's job (plan §5.3), not this property's.
    #[test]
    fn no_pre_existing_clobber(
        entries in snapshot(),
        p in support::policy_or_default(),
        oc in on_collision(),
        case in volume_case(),
    ) {
        let Some(plan) = planned(&entries, &p, oc, case) else { return Ok(()) };
        for item in renames(&plan) {
            let dest = key(&item.to, case);
            for e in &entries {
                if e.dir != item.dir {
                    continue;
                }
                if e.name == item.from {
                    continue; // its own source may legitimately be respelled
                }
                prop_assert_ne!(
                    key(&e.name, case), dest.clone(),
                    "{:?} -> {:?} would clobber existing {:?}", item.from, item.to, e.name
                );
            }
        }
    }

    /// **Order safety.** Applying the plan in the plan's own order never renames
    /// a directory before an item inside it.
    #[test]
    fn order_safety(
        entries in snapshot(),
        p in support::policy_or_default(),
        oc in on_collision(),
        case in volume_case(),
    ) {
        let Some(plan) = planned(&entries, &p, oc, case) else { return Ok(()) };
        for (i, earlier) in plan.items.iter().enumerate() {
            // Only a rename can invalidate a path, and only a directory has
            // anything inside it. An item that renames nothing cannot be "applied
            // too early" -- and `.`/`..`, which the generator produces because
            // they are corpus fixtures, are `Skipped` for exactly that reason.
            if earlier.resolution != Resolution::Rename || earlier.kind != EntryKind::Dir {
                continue;
            }
            let container = path_of(&earlier.dir, &earlier.from);
            for later in &plan.items[i + 1..] {
                prop_assert!(
                    !later.dir.starts_with(&container),
                    "{:?} is applied before {:?}, which lives inside it",
                    container, path_of(&later.dir, &later.from)
                );
            }
        }
    }

    /// **No sibling chains.** No `Rename` item's destination equals another
    /// `Rename` item's `from` in the same directory.
    ///
    /// §5.3 proves this cannot arise from an idempotent `transform`; the property
    /// is what makes the proof executable. The one exception is a name being
    /// *respelled* into its own comparison key (NFD -> NFC), which vacates
    /// nothing and is therefore not a chain -- see the note in `plan.rs`.
    #[test]
    fn no_sibling_chains(
        entries in snapshot(),
        p in support::policy_or_default(),
        oc in on_collision(),
        case in volume_case(),
    ) {
        let Some(plan) = planned(&entries, &p, oc, case) else { return Ok(()) };
        let sources: HashSet<(PathBuf, Vec<u8>)> = renames(&plan)
            .filter(|i| key(&i.from, case) != key(&i.to, case))
            .map(|i| (i.dir.clone(), key(&i.from, case)))
            .collect();
        for item in renames(&plan) {
            let k = (item.dir.clone(), key(&item.to, case));
            prop_assert!(
                !sources.contains(&k),
                "{:?} -> {:?} lands on another rename's source", item.from, item.to
            );
        }
    }

    /// **Bounded renumbering.** Every item either carries a destination inside
    /// both length limits or is not a `Rename` at all -- for every limit,
    /// including limits too small for any `-N` suffix.
    ///
    /// The 998-probe half of the statement is not observable from the outside
    /// (probing is not I/O and leaves no trace), so it is asserted in `plan.rs`'s
    /// own test module against the constant the loop uses.
    #[test]
    fn bounded_renumbering(
        entries in snapshot(),
        p in support::policy_strategy(),
        case in volume_case(),
    ) {
        let Some(plan) = planned(&entries, &p, OnCollision::Number, case) else { return Ok(()) };
        for item in &plan.items {
            if item.resolution == Resolution::Rename {
                let to = item.to.to_str().expect("a renamed destination is always text");
                prop_assert!(to.len() <= p.max_len_bytes, "{} bytes: {:?}", to.len(), to);
                prop_assert!(
                    to.chars().map(char::len_utf16).sum::<usize>() <= p.max_len_utf16,
                    "over the UTF-16 limit: {:?}", to
                );
            } else {
                // Nothing is renamed, so the name on disk is the one that was
                // already there -- never a guessed-at shorter one (§5.3).
                prop_assert_eq!(&item.to, &item.from);
            }
        }
    }

    /// **Fixed-point destinations (C-4).** Every `Rename` item's `to` is
    /// already what `transform` would produce from it -- not just the direct
    /// `transform` output (a fixed point by Idempotence, trivially), but also
    /// every numbered alternative the collision allocator hands back. A
    /// destination that is not a fixed point is one a subsequent run renames
    /// again, which is what silently invalidated a prior batch's undo.
    #[test]
    fn every_destination_is_a_fixed_point(
        entries in snapshot(),
        p in support::policy_or_default(),
        case in volume_case(),
    ) {
        let Some(plan) = planned(&entries, &p, OnCollision::Number, case) else { return Ok(()) };
        for item in renames(&plan) {
            let to = item.to.to_str().expect("a renamed destination is always text");
            match detoxrs_core::pipeline::transform(to, &p) {
                detoxrs_core::pipeline::TransformResult::Name(o) => {
                    prop_assert_eq!(&o.text, to, "{:?} is not a fixed point of transform", to);
                }
                detoxrs_core::pipeline::TransformResult::Unrepresentable(r) => {
                    prop_assert!(false, "{:?} -> Unrepresentable({r:?})", to);
                }
            }
        }
    }

    /// **Determinism.** Shuffling the input entry list produces an identical
    /// plan, including collision numbering. This is the executable form of
    /// detox's `readdir()`-order dependence being gone: which file "loses" a
    /// collision is a function of the names, never of the walk order.
    #[test]
    fn determinism(
        (entries, shuffled) in snapshot()
            .prop_flat_map(|v| (Just(v.clone()), Just(v).prop_shuffle())),
        p in support::policy_or_default(),
        oc in on_collision(),
        case in volume_case(),
    ) {
        prop_assert_eq!(
            plan(&entries, &p, oc, case),
            plan(&shuffled, &p, oc, case)
        );
    }
}

/// Named case for Determinism, because a property that only ever sees plans
/// without collisions would pass vacuously: this one pins *which* of two
/// colliding sources keeps the unnumbered name.
#[test]
fn numbering_follows_nfc_order_not_input_order() {
    let p = Policy::default();
    let mk = |name: &str, ino: u64| Entry {
        dir: PathBuf::from("t"),
        name: OsString::from(name),
        kind: EntryKind::File,
        ident: ident(ino),
        dir_ident: (1, 1),
        depth: 1,
    };
    // Both transform to `a_b.txt`. "a  b.txt" sorts first by NFC bytes (space <
    // 'b'), so it keeps the plain name in both input orders.
    let forward = vec![mk("a b.txt", 1), mk("a  b.txt", 2)];
    let reverse = vec![mk("a  b.txt", 2), mk("a b.txt", 1)];
    let a = plan(&forward, &p, OnCollision::Number, VolumeCase::Sensitive).expect("plan");
    let b = plan(&reverse, &p, OnCollision::Number, VolumeCase::Sensitive).expect("plan");
    assert_eq!(a, b);
    let dests: Vec<(&str, &str)> = a
        .items
        .iter()
        .map(|i| {
            (
                i.from.to_str().expect("ascii"),
                i.to.to_str().expect("ascii"),
            )
        })
        .collect();
    assert_eq!(
        dests,
        vec![("a  b.txt", "a_b.txt"), ("a b.txt", "a_b-2.txt")]
    );
}

/// The corpus, planned as one directory, must not error and must not produce two
/// identical destinations -- the end-to-end form of the collision engine's job
/// over the §8.3 fixture list rather than over generated names.
#[test]
fn the_corpus_plans_without_a_collision() {
    let entries: Vec<Entry> = support::corpus::all()
        .iter()
        .enumerate()
        .map(|(n, e)| Entry {
            dir: PathBuf::from("t"),
            name: os_string(&e.bytes),
            kind: EntryKind::File,
            ident: ident(n as u64),
            dir_ident: (1, 1),
            depth: 1,
        })
        .collect();
    for case in [VolumeCase::Sensitive, VolumeCase::Insensitive] {
        let plan = plan(&entries, &Policy::default(), OnCollision::Number, case).expect("plan");
        let mut seen: HashSet<Vec<u8>> = HashSet::new();
        for item in renames(&plan) {
            assert!(
                seen.insert(key(&item.to, case)),
                "corpus collision on {:?}",
                item.to
            );
        }
    }
}

#[cfg(unix)]
fn os_string(bytes: &[u8]) -> OsString {
    support::os_string_from_bytes(bytes)
}

#[cfg(not(unix))]
fn os_string(bytes: &[u8]) -> OsString {
    // Windows is best-effort tier: the invalid-UTF-8 fixtures cannot be built as
    // an `OsString` there at all, so they are represented by their escaped form.
    OsString::from(String::from_utf8_lossy(bytes).into_owned())
}
