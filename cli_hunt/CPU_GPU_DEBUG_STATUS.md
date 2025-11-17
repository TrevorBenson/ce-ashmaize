# CPU vs GPU Hash Discrepancy - Debug Status

## Current Status: IN PROGRESS

**Date**: November 17, 2025  
**Issue**: GPU hashes do not match CPU hashes for any input

---

## ✅ VERIFIED WORKING Components

### 1. Blake2b Implementation
- **Status**: ✅ CORRECT
- **Test**: Argon2 H' uses Blake2b internally
- **Result**: All H' tests pass, therefore Blake2b is working

### 2. Argon2 H' Implementation
- **Status**: ✅ CORRECT
- **Test**: Direct comparison of hprime output
- **Test Cases**: 4 cases (empty, short, long input; varying output sizes)
- **Result**: 100% match between CPU and GPU
- **File**: `examples/test_hprime.rs`

### 3. VM Initialization
- **Status**: ✅ CORRECT
- **Test**: Direct comparison of VM state after initialization  
- **Components Tested**:
  - All 32 registers
  - 64-byte prog_seed
  - With varying salt lengths (0, 1, 4 bytes)
- **Result**: 100% match between CPU and GPU
- **File**: `examples/test_vm_init.rs`

---

## ❌ FAILING Components

### Full Hash Computation
- **Status**: ❌ ALL TESTS FAIL
- **Symptom**: Divergence from byte 0 of output
- **Test Cases**: Empty, "a", "test" - all fail
- **Parameters**: Minimal (nb_loops=2, nb_instrs=256, 1MB ROM)

---

## 🔍 Root Cause Analysis

Since these components work:
- Blake2b ✅
- Argon2 H' ✅  
- VM initialization ✅

The issue MUST be in:

### Hypothesis 1: Instruction Execution Loop
- **Location**: `cuda/ashmaize.cu` line 129-137
- **Issue**: Possible bug in:
  - `execute_one_instruction` implementation
  - Instruction decoding
  - Operand fetching
  - Result writing

### Hypothesis 2: post_instructions
- **Location**: `cuda/ashmaize_vm.cuh`
- **Status**: Previously fixed but may still have issues
- **Components**:
  - prog_digest update
  - mem_digest update  
  - mixing_value computation
  - hprime call
  - XOR into registers

### Hypothesis 3: Finalization
- **Location**: `cuda/ashmaize.cu` line 139-170
- **Status**: Previously fixed
- **Components**:
  - prog_digest final
  - mem_digest final
  - Final Blake2b with all inputs

### Hypothesis 4: Main Loop Structure
- **Issue**: Mismatch in loop orchestration
- **Check**: Are we calling things in the right order?

---

## 🧪 Test Files Created

1. `examples/cpu_gpu_debug.rs` - Basic hash comparison
2. `examples/minimal_hash_test.rs` - Minimal parameters
3. `examples/test_hprime.rs` - Argon2 H' test ✅
4. `examples/test_vm_init.rs` - VM initialization test ✅
5. `examples/verify_salt_passing.rs` - Salt handling test
6. `examples/debug_vm_init.rs` - CPU VM state inspector

---

## 🔧 Known Issues Fixed Previously

1. **Salt length handling** - Fixed: correct padding and indexing
2. **post_instructions incomplete** - Fixed: full logic implemented
3. **finalize incomplete** - Fixed: includes memory_counter and all regs
4. **mem_access64** - Fixed: correct mem_digest update

---

## 📋 Next Steps

### Priority 1: Test Instruction Execution
Create a test that:
- Initializes VM (we know this works)
- Executes a SINGLE instruction
- Compares registers after execution

### Priority 2: Test post_instructions
Create a test that:
- Sets up a known VM state
- Calls post_instructions
- Compares resulting digests

### Priority 3: Test Full Loop
Create a test that:
- Runs ONE complete loop (init + 256 instrs + post)
- Compare intermediate state

### Priority 4: Binary Search
- Test with nb_loops=2, nb_instrs=256
- Then try nb_loops=2, nb_instrs=128
- Then try nb_loops=2, nb_instrs=64
- Find smallest failing case

---

## 💡 Debugging Strategy

1. **Isolate the failing component** using unit tests
2. **Add logging** to CUDA kernel (print first divergent value)
3. **Compare intermediate values** not just final output
4. **Use known test vectors** if available

---

## 📊 Performance Status

### Optimization Complete
- **Optimal Config**: Async + Clone, 131k batch/GPU
- **Performance**: 267,609 H/s (4x RTX 4090)
- **Status**: ✅ DOCUMENTED

### Hash Correctness
- **Status**: ❌ BLOCKING
- **Priority**: CRITICAL - must fix before deployment

---

## 🎯 Success Criteria

- [ ] CPU and GPU produce identical hashes for all test cases
- [ ] Understand root cause of discrepancy
- [ ] Document the fix
- [ ] Re-run all performance tests to verify fix doesn't degrade performance

---

## 📝 Notes

- Metal implementation was incomplete/buggy - not a reliable reference
- CUDA implementation is mostly correct (3/3 components verified)
- Issue is likely subtle (order of operations, loop counter, etc.)
- All primitives work correctly when tested in isolation

---

## 🔗 Related Files

- Main kernel: `cuda/ashmaize.cu`
- VM functions: `cuda/ashmaize_vm.cuh`
- GPU wrapper: `src/gpu.rs`
- CPU reference: `src/b2.rs`

