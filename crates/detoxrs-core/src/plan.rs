//! The collision engine (proposal §5.1, §5.3, §6.2; plan §7.2 work package 4).
//!
//! `plan()` is a pure function of a frozen walk snapshot. It performs **no I/O**:
//! no `std::fs`, no path existence check, no `stat`. That is the whole reason
//! §8.2's properties are cheap enough to run on every commit, and it is also the
//! safety argument -- detox renames a directory and then recurses into its new
//! path, which is the hazard its own maintainer named when rejecting the
//! force-overwrite request (`#130`): with `readdir()` order feeding renames as
//! they happen, which of N colliding files survives is filesystem-dependent.
//! Here the snapshot is frozen first, every destination is decided before
//! anything is written, and the allocation order is a total order over the source
//! names rather than the walk order.
//!
//! Three collision layers (§5.3), of which this module owns the first two:
//!
//! 1. **Intra-batch.** `(dir, comparison_key(to))` -> sources. More than one
//!    source on one destination is a conflict.
//! 2. **Pre-existing destination**, against the snapshot's own entries. The fresh
//!    `symlink_metadata` recheck at apply time is `apply.rs`'s half of this layer
//!    (plan §5.3), not this function's.
//! 3. **Kernel `RENAME_NOREPLACE`**, which is `fsops.rs`'s job entirely.
//!
//! The comparison key is **always NFC** and is case-folded **only** when the
//! caller says the volume is case-insensitive (§6.2). Case-insensitivity is an
//! input, never a per-OS assumption: macOS ships case-insensitive APFS by default
//! but case-sensitive APFS exists, and doc 06 tested both.

use crate::decode::{Decoded, decode};
use crate::pipeline::{TransformResult, Unrepresentable, transform};
use crate::policy::Policy;
use crate::truncate::{Limits, fits, split_extension, truncate_graphemes, utf16_len};
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use unicode_normalization::UnicodeNormalization as _;

/// What a snapshot entry is. Nothing here changes the transform; `apply` needs it
/// to pick the right syscall and the reporter needs it to print a trailing `/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// A regular file.
    File,
    /// A directory. Renaming one must never happen before an entry inside it.
    Dir,
    /// A symlink. Renamed as the link itself, never followed (§5.6).
    Symlink,
    /// FIFO, socket, device, or anything else.
    Other,
}

/// Identity captured at walk time, so `apply` can verify the entry it is about to
/// rename is still the entry that was planned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ident {
    /// Device number.
    pub dev: u64,
    /// Inode number.
    pub ino: u64,
    /// Link count, which is how a hardlink is recognised (§5.6).
    pub nlink: u64,
    /// Modification time.
    pub mtime: SystemTime,
}

/// A containing directory's identity.
///
/// `(dev, ino)`, or a path hash when identity is unavailable (`walk.rs`'s
/// `dir_ident_of` is the only producer). Named so the collision engine's maps
/// read as what they key on rather than as an anonymous pair of integers.
pub type DirIdent = (u64, u64);

/// One frozen directory entry: the input to `plan()`.
///
/// `name` is an `OsString`, not a `String`: a name that is not valid UTF-8 must
/// survive planning unharmed so it can be reported and skipped, never lossily
/// converted (§6.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The directory holding this entry. Never changes: detoxrs only ever
    /// rewrites a basename (§5.2).
    pub dir: PathBuf,
    /// The entry's current name.
    pub name: OsString,
    /// What kind of entry it is.
    pub kind: EntryKind,
    /// Identity at walk time.
    pub ident: Ident,
    /// Identity of `dir` itself: `(dev, ino)`, or a path hash when identity
    /// is unavailable (never faked; see `walk.rs`'s `dir_ident_of`).
    ///
    /// Two arguments that spell the same directory differently -- `.`, an
    /// empty string, `./x` -- still compare equal here even though `dir`
    /// does not (C8), which is what lets layer 1's `wants` and layer 2's
    /// `occupied` see both spellings as one collision universe instead of
    /// two.
    pub dir_ident: DirIdent,
    /// Depth below the walk root, carried for the report.
    ///
    /// Ordering does **not** trust this field: it is derived from `dir` instead,
    /// so Order safety cannot be broken by a walker that miscounts. See
    /// `deterministic_order`.
    pub depth: u32,
}

/// Why an entry is left alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// The name is not valid UTF-8. Skipped and reported, never repaired
    /// (owner decision, 2026-07-31).
    NotUtf8,
    /// The pipeline had no representable output (§3.14).
    Unrepresentable(Unrepresentable),
}

/// Why a destination could not be used.
///
/// Plan §7.2 sketched this as a single `Unresolvable` variant, commented "998
/// probes exhausted". Three variants ship instead, because under
/// `--on-collision skip` and `fail` no probing happens at all and reporting
/// "998 probes exhausted" for a plain two-file collision would be a false
/// statement in the one output a user reads before deciding to proceed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conflict {
    /// Two or more sources in this directory want this same destination. The
    /// executable form of the "N files collapse into 1" risk (§5.3).
    IntraBatch,
    /// The destination is already taken by a different entry in the snapshot.
    PreExisting,
    /// Renumbering exhausted its bound (§5.3): every `-N` for `N` in
    /// `2..=999` was either taken or too long to fit the limit.
    Unresolvable,
}

/// What the plan will do with an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// Rename `from` to `to`.
    Rename,
    /// Already clean. `to` equals `from`.
    Unchanged,
    /// Left alone with a reason. `to` equals `from`.
    Skipped(SkipReason),
    /// Wanted a destination it may not have. `to` equals `from`: a conflicted
    /// item is never renamed to a guessed-at name.
    Conflict(Conflict),
}

/// `--on-collision`. `Number` is the default by owner decision (2026-07-31).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OnCollision {
    /// Insert `-N` before the extension, smallest free `N >= 2`, bounded.
    #[default]
    Number,
    /// Leave every colliding entry alone and report it.
    Skip,
    /// Refuse the entire batch before renaming anything.
    Fail,
}

/// Whether the volume folds case when comparing names (§6.2).
///
/// An input rather than a `cfg!` on the platform, and an enum rather than a
/// `bool` so a call site cannot silently mean the opposite of what it reads. The
/// binary detects it empirically per volume (probe entry / `pathconf`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeCase {
    /// `A.txt` and `a.txt` are two different names.
    Sensitive,
    /// `A.txt` and `a.txt` are the same name, so renaming onto one clobbers the
    /// other.
    Insensitive,
}

/// One decided entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanItem {
    /// The containing directory.
    pub dir: PathBuf,
    /// The current name.
    pub from: OsString,
    /// The intended name. Equal to `from` for everything but `Rename`.
    pub to: OsString,
    /// What kind of entry it is.
    pub kind: EntryKind,
    /// Identity at walk time.
    pub ident: Ident,
    /// Depth below the walk root.
    pub depth: u32,
    /// What will happen.
    pub resolution: Resolution,
}

/// The whole decision, in apply order: deepest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Every snapshot entry, in the order `apply` must process them.
    pub items: Vec<PlanItem>,
}

/// Why there is no plan at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    /// `--on-collision fail` with at least one collision. There is deliberately
    /// no `Plan` in this case: "refuse the entire batch before renaming
    /// anything" (§5.3) is stronger when no applicable plan exists than when one
    /// exists and a caller is trusted to check a flag first. The conflicting
    /// items travel with the error so the reporter can still show what collided.
    BatchRefused(Vec<PlanItem>),
    /// The sibling-chain assertion fired (§5.3). Provably unreachable given the
    /// Idempotence property, so if this is ever seen, Idempotence broke first and
    /// the bug is in `pipeline.rs`, not here. Refuses the whole batch.
    InternalInconsistency(String),
}

/// First `-N` suffix tried, and the last. `2..=999` is 998 candidate probes per
/// source, which is §5.3's stated ceiling rather than a computed one: a directory
/// with a thousand names colliding on one destination is a case where the honest
/// output is a report, not a rename.
const FIRST_NUMBER: u32 = 2;
/// See [`FIRST_NUMBER`].
const LAST_NUMBER: u32 = 999;

/// Build a plan. No I/O, no panics, no filesystem access of any kind.
///
/// Items come back sorted deepest-first, ties broken by the NFC bytes of the
/// source name -- never the input order, which is what Determinism (§8.2)
/// asserts and what detox gets wrong.
///
/// # Errors
///
/// [`PlanError::BatchRefused`] when `on_collision` is [`OnCollision::Fail`] and
/// at least one destination collides. [`PlanError::InternalInconsistency`] when
/// the sibling-chain assertion fires, which means `transform` stopped being
/// idempotent.
pub fn plan(
    entries: &[Entry],
    p: &Policy,
    on_collision: OnCollision,
    case: VolumeCase,
) -> Result<Plan, PlanError> {
    let order = deterministic_order(entries);
    let desired: Vec<Desired> = order.iter().map(|&i| desired_for(&entries[i], p)).collect();

    check_no_sibling_chains(entries, &order, &desired)?;

    // Layer 1's count: how many sources want each destination. Only read by the
    // `skip`/`fail` arms; `number` gets the same answer out of the allocator,
    // because the second source to ask for a taken name is told it is taken.
    //
    // Keyed on `dir_ident`, not on `dir`'s text (C8): two arguments that spell
    // one directory differently must land in the same bucket, or a collision
    // between them is invisible to both layers at once.
    let mut wants: HashMap<(DirIdent, Vec<u8>), usize> = HashMap::new();
    for (pos, want) in desired.iter().enumerate() {
        if let Desired::Rename(text) = want {
            let dir_ident = entries[order[pos]].dir_ident;
            *wants
                .entry((dir_ident, key_of_text(text, case)))
                .or_insert(0) += 1;
        }
    }

    // Layer 2's occupancy: every entry occupies its current name, including the
    // ones being renamed away. Treating a to-be-vacated name as occupied is the
    // conservative half of §5.3's swap argument -- the non-conservative case
    // (this entry's destination is that entry's source, and that entry is also
    // moving) is exactly what `check_no_sibling_chains` has already refused.
    let mut allocator = Allocator {
        occupied: HashMap::new(),
        allocated: HashSet::new(),
        case,
        limits: Limits {
            bytes: p.max_len_bytes,
            utf16: p.max_len_utf16,
        },
        policy: *p,
    };
    for (pos, &i) in order.iter().enumerate() {
        allocator
            .occupied
            .entry((entries[i].dir_ident, key_of_os(&entries[i].name, case)))
            .or_default()
            .push(pos);
    }

    let mut items = Vec::with_capacity(entries.len());
    let mut refused = Vec::new();
    for (pos, &i) in order.iter().enumerate() {
        let e = &entries[i];
        let dir_ident = e.dir_ident;
        let (to, resolution) = match &desired[pos] {
            Desired::Keep => (e.name.clone(), Resolution::Unchanged),
            Desired::Skip(reason) => (e.name.clone(), Resolution::Skipped(*reason)),
            Desired::Rename(text) => {
                let key = key_of_text(text, case);
                match on_collision {
                    OnCollision::Number => allocator.take(dir_ident, text, pos).map_or_else(
                        || (e.name.clone(), Resolution::Conflict(Conflict::Unresolvable)),
                        |name| (OsString::from(name), Resolution::Rename),
                    ),
                    OnCollision::Skip | OnCollision::Fail => {
                        if allocator.is_free(dir_ident, &key, pos) {
                            if wants.get(&(dir_ident, key)).copied().unwrap_or(0) > 1 {
                                (e.name.clone(), Resolution::Conflict(Conflict::IntraBatch))
                            } else {
                                (OsString::from(text.clone()), Resolution::Rename)
                            }
                        } else {
                            (e.name.clone(), Resolution::Conflict(Conflict::PreExisting))
                        }
                    }
                }
            }
        };
        let item = PlanItem {
            dir: e.dir.clone(),
            from: e.name.clone(),
            to,
            kind: e.kind,
            ident: e.ident,
            depth: e.depth,
            resolution,
        };
        if on_collision == OnCollision::Fail && matches!(item.resolution, Resolution::Conflict(_)) {
            refused.push(item.clone());
        }
        items.push(item);
    }

    if refused.is_empty() {
        Ok(Plan { items })
    } else {
        Err(PlanError::BatchRefused(refused))
    }
}

/// What an entry wants, before any other entry is considered.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Desired {
    /// Already a fixed point of `transform`.
    Keep,
    /// No representable output, or no text at all.
    Skip(SkipReason),
    /// This name, if it can be had.
    Rename(String),
}

/// Stages 1 and 2-13 for one entry, and nothing else. Pure per entry: this is
/// what makes the collision layers a function of the snapshot alone.
fn desired_for(e: &Entry, p: &Policy) -> Desired {
    let Decoded::Utf8(text) = decode(&e.name) else {
        return Desired::Skip(SkipReason::NotUtf8);
    };
    match transform(&text, p) {
        TransformResult::Unrepresentable(r) => Desired::Skip(SkipReason::Unrepresentable(r)),
        TransformResult::Name(o) if o.text == text => Desired::Keep,
        TransformResult::Name(o) => Desired::Rename(o.text),
    }
}

/// The comparison key for a decoded name (§6.2).
fn comparison_key(text: &str, case: VolumeCase) -> String {
    let nfc: String = text.nfc().collect();
    match case {
        VolumeCase::Sensitive => nfc,
        // ponytail: `to_lowercase` stands in for full Unicode case folding. It
        // over-approximates (it can equate two names a real volume keeps apart,
        // which costs a spurious `-2`, never a lost file) and it is one line
        // instead of a case-folding table. Upgrade path: `unicode-case-mapping`
        // if a real volume's folding is ever measured to differ. NFC again after
        // folding because lowercasing is not normalization-preserving (U+0130).
        VolumeCase::Insensitive => nfc.to_lowercase().nfc().collect(),
    }
}

/// [`comparison_key`] as bytes, for a name that decoded.
fn key_of_text(text: &str, case: VolumeCase) -> Vec<u8> {
    comparison_key(text, case).into_bytes()
}

/// The comparison key for a raw name, decoded or not.
///
/// An undecodable name is never renamed, but it does *occupy* a name, so it needs
/// a key. `0xFF` cannot appear in UTF-8, so tagging the opaque keys with it makes
/// the two key spaces disjoint: no destination can ever be told it collides with
/// an opaque sibling that it does not actually collide with.
fn key_of_os(name: &OsStr, case: VolumeCase) -> Vec<u8> {
    name.to_str().map_or_else(
        || {
            let raw = name.as_encoded_bytes();
            let mut key = Vec::with_capacity(raw.len() + 1);
            key.push(0xFF);
            key.extend_from_slice(raw);
            key
        },
        |s| key_of_text(s, case),
    )
}

/// Apply order: deepest first, ties by NFC bytes of the source name.
///
/// Depth comes from `dir`'s component count rather than from `Entry::depth`.
/// Both should agree, but only one of them cannot lie, and Order safety is a
/// data-loss property: an entry inside a directory always has more path
/// components than the directory's own entry, so component count is exactly the
/// invariant the property needs.
fn deterministic_order(entries: &[Entry]) -> Vec<usize> {
    let mut keys: Vec<SortKey<'_>> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| SortKey {
            depth: Reverse(e.dir.components().count()),
            dir: e.dir.as_path(),
            nfc: key_of_os(&e.name, VolumeCase::Sensitive),
            raw: e.name.as_encoded_bytes(),
            index: i,
        })
        .collect();
    keys.sort_unstable();
    keys.into_iter().map(|k| k.index).collect()
}

/// The total order `plan()` walks in. Derived `Ord` is lexicographic by field
/// order, which is the order below, so the field order *is* the tie-break chain.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct SortKey<'a> {
    /// Deepest first, so a parent is never renamed before its contents.
    depth: Reverse<usize>,
    /// Group a directory's entries together.
    dir: &'a Path,
    /// §5.3's stated tie-break: the NFC bytes of the source name, *unfolded* --
    /// the order must not change with the volume's case sensitivity, or one tree
    /// would number its collisions differently on two mounts of itself.
    nfc: Vec<u8>,
    /// The raw bytes, and this field is a defect the Determinism property found
    /// rather than a flourish: §5.3 says "a stable sort order (NFC bytes of the
    /// source name)", but NFC is not injective over source names --
    /// `caf\u{e9}.txt` and `cafe\u{301}.txt` are two entries with one NFC key. A
    /// stable sort then leaves their relative order equal to the *input* order,
    /// i.e. `readdir()` order, which is the exact dependence the property exists
    /// to catch: with both spellings dirty, which one got `-2` depended on the
    /// walk.
    raw: &'a [u8],
    /// Only reachable when two entries have the same directory and the same raw
    /// name, which a real snapshot never contains. Present so the order is total.
    index: usize,
}

/// §5.3's assertion, on the *desired* destinations rather than the final ones.
///
/// If any `Rename`'s destination equals another `Rename`'s source in the same
/// directory, `transform` is not idempotent and the whole batch is refused. It is
/// checked before renumbering on purpose: renumbering would resolve the collision
/// and hide the defect, and the point of the assertion is that it fires loudly.
///
/// **A rename that only respells its own name is excluded**, and that exclusion
/// is load-bearing rather than cosmetic. `plan.rs`'s first draft did not have it
/// and a directory holding `cafe\u{301}.txt` (NFD) beside `caf\u{e9} .txt` fired
/// the assertion: the first entry's destination is its own NFC respelling, so it
/// vacates nothing, and the second entry landing on that key is an ordinary
/// intra-batch collision, not a chain. §5.3's prose ("the `from` of another item
/// that is also a `Rename`") reads as if it covers this case; taken literally it
/// reports an internal error for a snapshot two `cp` commands can produce.
fn check_no_sibling_chains(
    entries: &[Entry],
    order: &[usize],
    desired: &[Desired],
) -> Result<(), PlanError> {
    // Comparison here is exact NFC, never case-folded: the proof it defends is
    // about string equality of `transform`'s output, and folding would flag an
    // ordinary case-insensitive collision as an internal error.
    let nfc: Vec<String> = order
        .iter()
        .map(|&i| {
            entries[i]
                .name
                .to_str()
                .map(|s| s.nfc().collect())
                .unwrap_or_default()
        })
        .collect();

    let mut vacated: HashMap<(&Path, &str), usize> = HashMap::new();
    for (pos, want) in desired.iter().enumerate() {
        if let Desired::Rename(to) = want
            && nfc[pos] != *to
        {
            vacated.insert((entries[order[pos]].dir.as_path(), nfc[pos].as_str()), pos);
        }
    }

    for (pos, want) in desired.iter().enumerate() {
        let Desired::Rename(to) = want else { continue };
        let dir = entries[order[pos]].dir.as_path();
        if let Some(&other) = vacated.get(&(dir, to.as_str()))
            && other != pos
        {
            return Err(PlanError::InternalInconsistency(format!(
                "sibling rename chain in {}: {:?} -> {:?}, but {:?} is itself renamed. \
                     transform is not idempotent, so proposal 5.3's proof no longer holds; \
                     refusing the whole batch rather than renaming anything",
                dir.display(),
                nfc[pos],
                to,
                nfc[other],
            )));
        }
    }
    Ok(())
}

/// Destination allocation: layer 2's occupancy plus this run's own allocations.
///
/// Keyed on `(dir_ident, key)`, not `(dir, key)` (C8): `dir_ident` is the
/// containing directory's identity, so two arguments that spell one directory
/// differently still land in the same bucket. This also drops the lifetime a
/// `&Path` key would need, since `DirIdent` is `Copy`.
struct Allocator {
    /// `(dir_ident, key)` -> the positions of the entries currently holding
    /// that name.
    occupied: HashMap<(DirIdent, Vec<u8>), Vec<usize>>,
    /// `(dir_ident, key)` already promised to an earlier item in the plan's
    /// order.
    allocated: HashSet<(DirIdent, Vec<u8>)>,
    /// Whether comparison folds case.
    case: VolumeCase,
    /// The length budget every candidate must satisfy.
    limits: Limits,
    /// The policy `numbered()`'s candidates are checked against (C-4):
    /// `transform` is the sole authority on what is safe, so a hand-built
    /// candidate is only trusted after `transform` agrees it is already its
    /// own fixed point.
    policy: Policy,
}

impl Allocator {
    /// Is `key` available to the entry at `owner`?
    ///
    /// An entry's own name is available to itself: that is the NFD -> NFC respell
    /// case (§6.2), where the destination's key equals the source's key but the
    /// bytes on disk change. Reporting that as a conflict would refuse to fix
    /// exactly the normalization mess the tool exists for.
    fn is_free(&self, dir_ident: DirIdent, key: &[u8], owner: usize) -> bool {
        // The tuple key means one `Vec` per probe. Bounded by the probe ceiling
        // and dwarfed by the syscall it exists to avoid; a nested map would save
        // it and cost more code than it is worth.
        let k = (dir_ident, key.to_vec());
        !self.allocated.contains(&k)
            && self
                .occupied
                .get(&k)
                .is_none_or(|holders| holders.iter().all(|&h| h == owner))
    }

    /// The smallest free candidate for `want`, or `None` if there is none.
    ///
    /// Order matters and is the reason `plan` walks `deterministic_order`: the
    /// first source to ask keeps the unnumbered name.
    fn take(&mut self, dir_ident: DirIdent, want: &str, owner: usize) -> Option<String> {
        let key = key_of_text(want, self.case);
        if self.is_free(dir_ident, &key, owner) {
            self.allocated.insert((dir_ident, key));
            return Some(want.to_owned());
        }
        for n in FIRST_NUMBER..=LAST_NUMBER {
            // `None` from `numbered` is not "try the next N": a longer suffix
            // is never shorter, so if `-2` does not fit the limit, nothing
            // does. Bailing here is what keeps the probe count at or below the
            // 998 ceiling instead of spinning through all of them at a 2-byte
            // limit.
            let Some(candidate) = numbered(want, n, &self.limits) else {
                break;
            };
            // C-4: string surgery is not proof. `numbered` inserts `-N` before
            // the extension by construction, never by running the result back
            // through `transform` -- so when truncation leaves the kept stem
            // ending in `-`, the appended `-N` manufactures a `--` run that
            // stage 9 later collapses, and the destination this function
            // handed back was never a fixed point of `transform` at all. A
            // non-fixed-point destination is the one thing §5.3's whole
            // safety argument assumes cannot happen: it is a name a
            // subsequent run renames again, which is exactly what silently
            // invalidates the batch that produced it. Unlike a too-long
            // candidate, a non-fixed-point one does *not* imply every larger
            // `N` is equally bad (a different suffix length truncates the
            // stem to a different length and may not end in `-`), so this
            // continues the probe instead of breaking it.
            if !is_fixed_point(&candidate, &self.policy) {
                continue;
            }
            let key = key_of_text(&candidate, self.case);
            if self.is_free(dir_ident, &key, owner) {
                self.allocated.insert((dir_ident, key));
                return Some(candidate);
            }
        }
        None
    }
}

/// Is `candidate` already what `transform` would produce from it?
///
/// The invariant every destination `plan()` emits must satisfy (C-4): the
/// direct `transform` output is a fixed point by Idempotence, but a
/// hand-built candidate such as `numbered()`'s is not exempt from the same
/// check just because it was built to fit the length limit.
fn is_fixed_point(candidate: &str, p: &Policy) -> bool {
    matches!(transform(candidate, p), TransformResult::Name(o) if o.text == candidate)
}

/// `want` with `-N` inserted before the extension, truncated to fit both limits.
///
/// `None` when no `-N` form fits at all: the suffix and extension alone exhaust
/// the budget, or truncating the stem to make room would empty it. §5.3 forbids
/// the alternatives outright -- we never drop the numbering to fit and never
/// exceed the limit, so "no name" is the honest answer and the caller turns it
/// into a `Conflict`.
fn numbered(want: &str, n: u32, limits: &Limits) -> Option<String> {
    let (stem, ext) = split_extension(want);
    let suffix = format!("-{n}");
    let budget = Limits {
        bytes: limits.bytes.checked_sub(suffix.len() + ext.len())?,
        utf16: limits
            .utf16
            .checked_sub(utf16_len(&suffix) + utf16_len(ext))?,
    };
    let kept = truncate_graphemes(stem, &budget);
    if kept.is_empty() {
        return None;
    }
    let out = format!("{kept}{suffix}{ext}");
    debug_assert!(
        fits(&out, limits),
        "numbered candidate over the limit: {out}"
    );
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::{
        Conflict, Desired, Entry, EntryKind, Ident, LAST_NUMBER, OnCollision, Plan, PlanError,
        Resolution, SkipReason, VolumeCase, check_no_sibling_chains, numbered, plan,
    };
    use crate::pipeline::{TransformResult, Unrepresentable, transform};
    use crate::policy::Policy;
    use crate::truncate::Limits;
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn dir() -> PathBuf {
        PathBuf::from("t")
    }

    fn entry(name: &str) -> Entry {
        Entry {
            dir: dir(),
            name: OsString::from(name),
            kind: EntryKind::File,
            ident: Ident {
                dev: 1,
                ino: 1,
                nlink: 1,
                mtime: SystemTime::UNIX_EPOCH,
            },
            dir_ident: (1, 1),
            depth: 1,
        }
    }

    fn run(names: &[&str], oc: OnCollision) -> Plan {
        let entries: Vec<Entry> = names.iter().map(|n| entry(n)).collect();
        plan(&entries, &Policy::default(), oc, VolumeCase::Sensitive).expect("plan")
    }

    /// `(from, to, resolution)` in the plan's own order.
    fn rows(p: &Plan) -> Vec<(String, String, Resolution)> {
        p.items
            .iter()
            .map(|i| {
                (
                    i.from.to_string_lossy().into_owned(),
                    i.to.to_string_lossy().into_owned(),
                    i.resolution,
                )
            })
            .collect()
    }

    #[test]
    fn a_clean_directory_renames_nothing() {
        let p = run(&["a_b.txt", "c.txt"], OnCollision::Number);
        assert!(
            p.items
                .iter()
                .all(|i| i.resolution == Resolution::Unchanged)
        );
    }

    /// Layer 1: two sources, one destination. The NFC-first source keeps the
    /// plain name; the other is numbered.
    #[test]
    fn two_sources_on_one_destination_are_numbered() {
        let p = run(&["a b.txt", "a  b.txt"], OnCollision::Number);
        assert_eq!(
            rows(&p),
            vec![
                ("a  b.txt".into(), "a_b.txt".into(), Resolution::Rename),
                ("a b.txt".into(), "a_b-2.txt".into(), Resolution::Rename),
            ]
        );
    }

    /// Layer 2: the destination is an entry that already exists and is not moving.
    #[test]
    fn a_pre_existing_destination_is_numbered_around() {
        let p = run(&["a b.txt", "a_b.txt"], OnCollision::Number);
        assert_eq!(
            rows(&p),
            vec![
                (
                    "a b.txt".to_owned(),
                    "a_b-2.txt".to_owned(),
                    Resolution::Rename
                ),
                (
                    "a_b.txt".to_owned(),
                    "a_b.txt".to_owned(),
                    Resolution::Unchanged
                ),
            ]
        );
    }

    /// Three sources on one destination: `-2` then `-3`, never a gap and never a
    /// reuse.
    #[test]
    fn numbering_fills_the_smallest_free_slot() {
        let p = run(&["a b.txt", "a  b.txt", "a   b.txt"], OnCollision::Number);
        let dests: Vec<String> = p
            .items
            .iter()
            .map(|i| i.to.to_string_lossy().into_owned())
            .collect();
        assert_eq!(dests, vec!["a_b.txt", "a_b-2.txt", "a_b-3.txt"]);
    }

    /// An existing `-2` is not stolen: the loser skips to `-3`.
    #[test]
    fn an_existing_numbered_name_is_respected() {
        let p = run(&["a b.txt", "a_b.txt", "a_b-2.txt"], OnCollision::Number);
        let renamed: Vec<String> = p
            .items
            .iter()
            .filter(|i| i.resolution == Resolution::Rename)
            .map(|i| i.to.to_string_lossy().into_owned())
            .collect();
        assert_eq!(renamed, vec!["a_b-3.txt"]);
    }

    #[test]
    fn skip_leaves_both_sides_alone() {
        let p = run(&["a b.txt", "a_b.txt"], OnCollision::Skip);
        assert!(
            p.items.iter().any(|i| i.from == *"a b.txt"
                && i.resolution == Resolution::Conflict(Conflict::PreExisting))
        );
        // Nothing is renamed, and the conflicted item keeps its own name.
        for i in &p.items {
            assert_eq!(i.from, i.to);
        }
    }

    #[test]
    fn skip_reports_an_intra_batch_collision_on_both_sources() {
        let p = run(&["a b.txt", "a  b.txt"], OnCollision::Skip);
        for i in &p.items {
            assert_eq!(i.resolution, Resolution::Conflict(Conflict::IntraBatch));
            assert_eq!(i.from, i.to);
        }
    }

    #[test]
    fn fail_refuses_the_whole_batch_and_returns_no_plan() {
        let entries: Vec<Entry> = ["a b.txt", "a  b.txt", "clean.txt", "d e.txt"]
            .iter()
            .map(|n| entry(n))
            .collect();
        let err = plan(
            &entries,
            &Policy::default(),
            OnCollision::Fail,
            VolumeCase::Sensitive,
        )
        .expect_err("a collision under `fail` must refuse the batch");
        match err {
            PlanError::BatchRefused(items) => {
                assert_eq!(items.len(), 2);
                assert!(items.iter().all(|i| i.from == i.to));
            }
            PlanError::InternalInconsistency(m) => panic!("{m}"),
        }
    }

    #[test]
    fn fail_with_no_collision_still_plans() {
        let p = run(&["a b.txt", "d e.txt"], OnCollision::Fail);
        assert_eq!(
            p.items
                .iter()
                .filter(|i| i.resolution == Resolution::Rename)
                .count(),
            2
        );
    }

    /// §6.2: folding happens only when the caller says the volume folds.
    #[test]
    fn case_folding_is_the_callers_call() {
        let entries: Vec<Entry> = ["A B.txt", "a_b.txt"].iter().map(|n| entry(n)).collect();
        let p = &Policy::default();
        let sensitive =
            plan(&entries, p, OnCollision::Number, VolumeCase::Sensitive).expect("plan");
        let insensitive =
            plan(&entries, p, OnCollision::Number, VolumeCase::Insensitive).expect("plan");
        // Case-sensitive: `A_B.txt` and `a_b.txt` are different names.
        assert_eq!(
            sensitive
                .items
                .iter()
                .find(|i| i.from == *"A B.txt")
                .map(|i| i.to.to_string_lossy().into_owned()),
            Some("A_B.txt".to_owned())
        );
        // Case-insensitive: renaming onto `a_b.txt` would clobber it.
        assert_eq!(
            insensitive
                .items
                .iter()
                .find(|i| i.from == *"A B.txt")
                .map(|i| i.to.to_string_lossy().into_owned()),
            Some("A_B-2.txt".to_owned())
        );
    }

    /// §6.2's NFD -> NFC respell: the destination's comparison key equals the
    /// source's, which is not a collision with itself.
    #[test]
    fn an_nfd_to_nfc_respell_is_a_rename_not_a_conflict() {
        let p = run(&["cafe\u{301}.txt"], OnCollision::Number);
        assert_eq!(
            rows(&p),
            vec![(
                "cafe\u{301}.txt".to_owned(),
                "caf\u{e9}.txt".to_owned(),
                Resolution::Rename
            )]
        );
    }

    /// The defect the Determinism property found, pinned as a named case: two
    /// spellings of one NFC key are two entries, so "sort by NFC bytes of the
    /// source name" (§5.3) is not a total order and a stable sort falls back to
    /// `readdir()` order for exactly the pair whose numbering is contested.
    #[test]
    fn an_nfc_tie_is_broken_by_the_raw_bytes_not_by_input_order() {
        let p = Policy::default();
        let mk = |name: &str| entry(name);
        // Both are dirty and both want `caf\u{e9}.txt`, and their NFC source keys
        // are identical, so only the raw bytes can decide who is numbered.
        let forward: Vec<Entry> = ["cafe\u{301} .txt", "caf\u{e9} .txt"].map(mk).into();
        let reverse: Vec<Entry> = ["caf\u{e9} .txt", "cafe\u{301} .txt"].map(mk).into();
        let a = plan(&forward, &p, OnCollision::Number, VolumeCase::Sensitive).expect("plan");
        let b = plan(&reverse, &p, OnCollision::Number, VolumeCase::Sensitive).expect("plan");
        assert_eq!(a, b);
        assert_eq!(
            rows(&a),
            vec![
                (
                    "cafe\u{301} .txt".to_owned(),
                    "caf\u{e9}.txt".to_owned(),
                    Resolution::Rename
                ),
                (
                    "caf\u{e9} .txt".to_owned(),
                    "caf\u{e9}-2.txt".to_owned(),
                    Resolution::Rename
                ),
            ]
        );
    }

    /// The case that made `check_no_sibling_chains` exclude respells: a respelled
    /// entry vacates nothing, so a second entry landing on its key is an ordinary
    /// intra-batch collision, not an internal error.
    #[test]
    fn a_respell_beside_a_colliding_sibling_is_not_a_chain() {
        let p = run(&["cafe\u{301}.txt", "caf\u{e9} .txt"], OnCollision::Number);
        let dests: Vec<String> = p
            .items
            .iter()
            .map(|i| i.to.to_string_lossy().into_owned())
            .collect();
        // `caf\u{e9} .txt` sorts first (space < '.') and finds the NFC key
        // already occupied by the NFD entry, so it takes `-2`; the NFD entry then
        // respells into the key it already owned. No internal error, no clobber.
        assert_eq!(dests, vec!["caf\u{e9}-2.txt", "caf\u{e9}.txt"]);
    }

    #[test]
    fn an_undecodable_name_is_skipped_never_renamed() {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt as _;
            let mut e = entry("placeholder");
            e.name = OsString::from_vec(b"bad\xffname.txt".to_vec());
            let p = plan(
                &[e],
                &Policy::default(),
                OnCollision::Number,
                VolumeCase::Sensitive,
            )
            .expect("plan");
            assert_eq!(
                p.items[0].resolution,
                Resolution::Skipped(SkipReason::NotUtf8)
            );
            assert_eq!(p.items[0].from, p.items[0].to);
        }
    }

    #[test]
    fn an_unrepresentable_name_is_skipped_never_renamed() {
        let p = run(&["***", "clean.txt"], OnCollision::Number);
        assert_eq!(
            p.items
                .iter()
                .find(|i| i.from == *"***")
                .map(|i| i.resolution),
            Some(Resolution::Skipped(SkipReason::Unrepresentable(
                Unrepresentable::ReducesToEmpty
            )))
        );
    }

    /// Order safety, as a named case rather than only as a property: a dirty
    /// directory name and an entry inside it.
    #[test]
    fn a_parent_directory_is_renamed_after_its_contents() {
        let mut parent = entry("a b");
        parent.kind = EntryKind::Dir;
        let mut child = entry("c d.txt");
        child.dir = dir().join("a b");
        child.dir_ident = (1, 2); // a different directory than `dir()`
        child.depth = 2;
        let p = plan(
            &[parent, child],
            &Policy::default(),
            OnCollision::Number,
            VolumeCase::Sensitive,
        )
        .expect("plan");
        let order: Vec<String> = p
            .items
            .iter()
            .map(|i| i.dir.join(&i.from).to_string_lossy().into_owned())
            .collect();
        assert_eq!(order, vec!["t/a b/c d.txt", "t/a b"]);
    }

    // ---- the sibling swap ----------------------------------------------------

    /// §5.3's proof, exercised rather than quoted: for a swap you need
    /// `f(a) = b` and `f(b) = a` with `a != b`, and Idempotence
    /// (`f(f(a)) = f(a)`) makes that impossible. So for every name the pipeline
    /// changes, the destination is already a fixed point -- which is why the
    /// *other* member of any near-swap pair comes out `Unchanged` and the case
    /// degrades to an ordinary pre-existing-destination conflict.
    #[test]
    fn a_swap_cannot_be_constructed() {
        let p = Policy::default();
        for a in [
            "a b",
            "a-b",
            "a_b",
            "A.txt",
            "a.txt",
            "x 2",
            "x-2",
            "IMG 0042.JPG",
            "IMG_0042.JPG",
            "cafe\u{301}.txt",
            "caf\u{e9}.txt",
            ".hidden file",
            "report .tar.gz",
        ] {
            let TransformResult::Name(b) = transform(a, &p) else {
                continue;
            };
            if b.text == a {
                continue; // `a` is a fixed point: nothing to swap.
            }
            // The would-be partner is `b`. For a swap, `transform(b)` would have
            // to be `a`; Idempotence forces it to be `b`.
            match transform(&b.text, &p) {
                TransformResult::Name(again) => assert_eq!(
                    again.text, b.text,
                    "{a:?} -> {:?} -> {:?} is a swap, so Idempotence broke",
                    b.text, again.text
                ),
                TransformResult::Unrepresentable(r) => {
                    panic!("{a:?} -> {:?} -> Unrepresentable({r:?})", b.text)
                }
            }
        }
    }

    /// And the assertion is loud when handed the thing it exists to catch. This
    /// is the only way to see it fire: `plan()` cannot be made to produce a chain
    /// through its own pipeline, so the checker is fed a hand-built pair of
    /// desires that a non-idempotent `transform` would have produced.
    #[test]
    fn the_chain_assertion_fires_on_a_hand_built_swap() {
        let entries = vec![entry("a"), entry("b")];
        let order = vec![0, 1];
        let desired = vec![
            Desired::Rename("b".to_owned()),
            Desired::Rename("a".to_owned()),
        ];
        match check_no_sibling_chains(&entries, &order, &desired) {
            Err(PlanError::InternalInconsistency(msg)) => {
                assert!(msg.contains("sibling rename chain"), "{msg}");
                assert!(msg.contains("not idempotent"), "{msg}");
            }
            other => panic!("the swap was not refused: {other:?}"),
        }
    }

    /// A one-way chain (`a -> b`, `b -> c`) is refused too, not just a two-cycle.
    #[test]
    fn the_chain_assertion_fires_on_a_one_way_chain() {
        let entries = vec![entry("a"), entry("b")];
        let order = vec![0, 1];
        let desired = vec![
            Desired::Rename("b".to_owned()),
            Desired::Rename("c".to_owned()),
        ];
        assert!(matches!(
            check_no_sibling_chains(&entries, &order, &desired),
            Err(PlanError::InternalInconsistency(_))
        ));
    }

    // ---- bounded renumbering -------------------------------------------------

    /// §5.3's ceiling, against the constant the loop actually uses: `2..=999` is
    /// 998 candidate probes per source, and no more.
    #[test]
    fn the_probe_ceiling_is_998() {
        assert_eq!(
            usize::try_from(LAST_NUMBER - super::FIRST_NUMBER + 1),
            Ok(998)
        );
    }

    #[test]
    fn a_limit_too_small_for_any_suffix_yields_a_conflict() {
        // 1 byte: `a b` and `a c` both reduce to `a`, and `-2` needs 2 bytes
        // before the stem, so no numbered candidate exists at all.
        let p = Policy {
            separator: '_',
            max_len_bytes: 1,
            max_len_utf16: 1,
        };
        let entries: Vec<Entry> = ["a b", "a c"].iter().map(|n| entry(n)).collect();
        let plan = plan(&entries, &p, OnCollision::Number, VolumeCase::Sensitive).expect("plan");
        assert!(
            plan.items
                .iter()
                .any(|i| i.resolution == Resolution::Conflict(Conflict::Unresolvable))
        );
        for i in &plan.items {
            if matches!(i.resolution, Resolution::Conflict(_)) {
                assert_eq!(i.from, i.to, "a conflicted item must keep its own name");
            }
        }
    }

    /// C-4 / O1-1's minimal reproduction: at a tight limit, the first numbered
    /// candidate the naive construction would try (`-2`) truncates the stem to
    /// something ending in `-`, and appending `-2` there manufactures a `--`
    /// run that `transform` itself would collapse. The allocator must not hand
    /// that name back; it must keep probing until it finds one `transform`
    /// agrees is already a fixed point.
    #[test]
    fn a_numbered_destination_is_always_a_fixed_point() {
        let p = Policy::new('_', 9, 9).expect("'_' is Keep-class");
        let entries: Vec<Entry> = ["ab-cd .txt", "ab-cd.txt"]
            .iter()
            .map(|n| entry(n))
            .collect();
        let plan = plan(&entries, &p, OnCollision::Number, VolumeCase::Sensitive).expect("plan");
        for item in &plan.items {
            if item.resolution != Resolution::Rename {
                continue;
            }
            let to = item.to.to_str().expect("ascii");
            match transform(to, &p) {
                TransformResult::Name(o) => assert_eq!(
                    o.text, to,
                    "{to:?} is not a fixed point of transform (from {:?})",
                    item.from
                ),
                TransformResult::Unrepresentable(r) => {
                    panic!("{to:?} -> Unrepresentable({r:?})")
                }
            }
        }
    }

    #[test]
    fn numbering_truncates_the_stem_rather_than_exceeding_the_limit() {
        let limits = Limits {
            bytes: 10,
            utf16: 10,
        };
        assert_eq!(
            numbered("abcdefgh.txt", 2, &limits).as_deref(),
            Some("abcd-2.txt")
        );
        assert_eq!(
            numbered("abcdefgh.txt", 999, &limits).as_deref(),
            Some("ab-999.txt")
        );
        // The extension alone leaves no room for a stem plus a suffix.
        assert_eq!(numbered("a.verylongext", 2, &limits), None);
    }
}
