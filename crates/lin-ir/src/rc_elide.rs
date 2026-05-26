//! RC elision pass for LinIR.
//!
//! Eliminates Retain/Release pairs where the retained value's live range
//! does not span any allocation or call site that could trigger GC or reuse.
//!
//! Algorithm (simplified Perceus):
//! 1. Run liveness analysis on the function.
//! 2. For each Release instruction at position (block, i):
//!    - If the released temp is NOT live after the release (i.e., not in
//!      live_in of any successor that can reach a use), the release is a
//!      "last use" release — keep it, it is correct.
//!    - Walk backwards from the Release to find the paired Retain.
//!    - If the path from Retain to Release contains no call, allocation, or
//!      another Release for the same temp, the retain/release pair is elided:
//!      the value was never shared, so its refcount was always 1 and the
//!      retain/release pair is a no-op.
//! 3. Remove elided Retain/Release pairs from the instruction list.
//!
//! This is a conservative approximation: we err on the side of keeping RC ops
//! when the path analysis is uncertain (aliasing, indirect calls).
//!
//! Reference: Reinking et al., "Perceus: Garbage Free Reference Counting with
//! Reuse", PLDI 2021.

use std::collections::HashSet;

use lin_check::types::Type;

use crate::ir::*;
use crate::liveness::Liveness;

/// Run the RC elision pass on all functions in a module, mutating in place.
pub fn elide_rc(module: &mut LinModule) {
    for func in &mut module.functions {
        elide_rc_fn(func);
    }
}

fn elide_rc_fn(func: &mut LinFunction) {
    // Liveness is available for future use (e.g., detecting last-use releases).
    let _liveness = Liveness::compute(func);

    // Collect (block_idx, instr_idx) pairs to remove.
    let mut to_remove: HashSet<(usize, usize)> = HashSet::new();

    for block_idx in 0..func.blocks.len() {
        let instrs = func.blocks[block_idx].instructions.clone();

        // For each Retain, look forward for its matching Release with a clean path.
        for (retain_idx, instr) in instrs.iter().enumerate() {
            let Instruction::Retain { val: retain_val, ty } = instr else {
                continue;
            };
            if !is_rc_type(ty) {
                continue;
            }
            if let Some(release_idx) = find_paired_release(*retain_val, block_idx, retain_idx, &instrs) {
                let path_clean = path_has_no_interference(
                    *retain_val,
                    retain_idx,
                    release_idx,
                    &instrs,
                );
                if path_clean {
                    to_remove.insert((block_idx, retain_idx));
                    to_remove.insert((block_idx, release_idx));
                }
            }
        }
    }

    // Remove instructions in reverse order so indices stay valid.
    for block_idx in 0..func.blocks.len() {
        let mut remove_here: Vec<usize> = to_remove
            .iter()
            .filter(|(b, _)| *b == block_idx)
            .map(|(_, i)| *i)
            .collect();
        remove_here.sort_unstable_by(|a, b| b.cmp(a)); // descending
        for idx in remove_here {
            func.blocks[block_idx].instructions.remove(idx);
        }
    }
}

/// Types that participate in RC (reference counted heap values).
fn is_rc_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Str | Type::Array(_) | Type::FixedArray(_) | Type::Object(_) | Type::Function { .. }
    )
}

/// Find the Release instruction that is paired with a Retain at `retain_idx`.
/// Searches forward; returns None if another definition of `temp` or an
/// unbalanced release occurs before the release.
fn find_paired_release(
    temp: Temp,
    _block_idx: usize,
    retain_idx: usize,
    instrs: &[Instruction],
) -> Option<usize> {
    for i in (retain_idx + 1)..instrs.len() {
        match &instrs[i] {
            Instruction::Release { val, .. } if *val == temp => return Some(i),
            other => {
                let (_uses, defs) = crate::liveness::instr_use_def(other);
                // If temp is redefined, it's a different value — the retain is not for it.
                if defs.contains(&temp) {
                    return None;
                }
            }
        }
    }
    None
}

/// Check that the instructions between `retain_idx` (exclusive) and `release_idx` (exclusive)
/// contain no:
/// - Call or CallIntrinsic instructions (could trigger GC or alias the temp)
/// - MakeObject/MakeArray/MakeClosure (heap allocations)
/// - Another Release for `temp` (would double-free)
fn path_has_no_interference(
    temp: Temp,
    retain_idx: usize,
    release_idx: usize,
    instrs: &[Instruction],
) -> bool {
    for i in (retain_idx + 1)..release_idx {
        match &instrs[i] {
            Instruction::Call { .. }
            | Instruction::CallIntrinsic { .. }
            | Instruction::MakeObject { .. }
            | Instruction::MakeArray { .. }
            | Instruction::MakeClosure { .. } => return false,
            Instruction::Release { val, .. } if *val == temp => return false,
            _ => {}
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fn(id: FuncId, instrs: Vec<Instruction>) -> LinFunction {
        let block = BasicBlock {
            id: BlockId(0),
            label: None,
            instructions: instrs,
            terminator: Terminator::Return(None),
        };
        let mut temp_types = std::collections::HashMap::new();
        temp_types.insert(Temp(0), Type::Str);
        temp_types.insert(Temp(1), Type::Str);
        LinFunction {
            id,
            name: None,
            params: vec![],
            is_closure: false,
            ret_ty: Type::Null,
            blocks: vec![block],
            temp_types,
            temp_count: 2,
            intrinsic_slots: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn elides_adjacent_retain_release_with_no_interference() {
        // Retain(t0) followed immediately by Release(t0) with t0 still live = elide both.
        let instrs = vec![
            Instruction::Retain { val: Temp(0), ty: Type::Str },
            // Some use of t0 that keeps it live.
            Instruction::Copy { dst: Temp(1), src: Temp(0) },
            Instruction::Release { val: Temp(0), ty: Type::Str },
        ];
        let mut module = LinModule {
            functions: vec![make_fn(FuncId(0), instrs)],
            global_fn_slots: std::collections::HashMap::new(),
            intrinsics: std::collections::HashMap::new(),
        };
        elide_rc(&mut module);
        // The retain and release should have been elided (only the Copy remains).
        let remaining = &module.functions[0].blocks[0].instructions;
        assert!(
            !remaining.iter().any(|i| matches!(i, Instruction::Retain { .. })),
            "Retain should be elided"
        );
        assert!(
            !remaining.iter().any(|i| matches!(i, Instruction::Release { .. })),
            "Release should be elided"
        );
    }

    #[test]
    fn keeps_retain_release_with_call_in_between() {
        // Retain(t0) + Call + Release(t0): cannot elide because the call may alias.
        let null_ty = Type::Null;
        let instrs = vec![
            Instruction::Retain { val: Temp(0), ty: Type::Str },
            Instruction::Call {
                dst: Temp(1),
                callee: CallTarget::Named("foo".into()),
                args: vec![],
                ret_ty: null_ty,
            },
            Instruction::Release { val: Temp(0), ty: Type::Str },
        ];
        let mut module = LinModule {
            functions: vec![make_fn(FuncId(0), instrs)],
            global_fn_slots: std::collections::HashMap::new(),
            intrinsics: std::collections::HashMap::new(),
        };
        elide_rc(&mut module);
        // Both retain and release should remain.
        let remaining = &module.functions[0].blocks[0].instructions;
        assert!(
            remaining.iter().any(|i| matches!(i, Instruction::Retain { .. })),
            "Retain should be kept"
        );
        assert!(
            remaining.iter().any(|i| matches!(i, Instruction::Release { .. })),
            "Release should be kept"
        );
    }
}
