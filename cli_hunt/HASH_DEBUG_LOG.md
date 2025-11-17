# Hash Correctness Debug Log

## Session: November 17, 2025

### Attempts

#### Attempt 1: Check loop_counter increment location
- **Hypothesis**: loop_counter not being incremented
- **Finding**: Both CPU and GPU increment in `post_instructions` - NOT the issue
- **Result**: ✗ No change

#### Attempt 2: loop_counter encoding size
- **Hypothesis**: loop_counter encoded as 8 bytes instead of 4
- **Finding**: CPU uses `to_le_bytes()` on u32 (4 bytes), GPU was encoding as u64 (8 bytes)
- **Fix**: Changed mixing_input from 136 bytes to 132 bytes, loop_counter from 8 bytes to 4 bytes
- **Result**: ✗ Hash changed but still doesn't match

### Current Status
- Hashes still diverge from byte 0
- But the fix DID change the output (55d4... instead of 62a2...)
- This suggests we're getting closer

### Next Investigation Areas

1. **Check ip field usage** - GPU sets vm.ip = 0 at end of post_instructions, does CPU?
2. **Verify program generation** - Is hprime being called the same way?
3. **Check instruction pointer wrapping** - CPU might wrap ip differently
4. **Verify execute_one_instruction** - Need to compare implementation detail by detail

### Test Results After loop_counter Fix

```
Test: 'empty'
  CPU: f4778900bb638b14942c306bd6b09194
  GPU: 412bd2012bf5ebc3d62f5e3526e7b464

Test: 'a'
  CPU: 3fbd2c58592377911088d40d52611dc9
  GPU: 3bcb874f6a40e5eaf702246be07a5a6e

Test: 'test'
  CPU: ccc147a1fefff52af88cfbca9cc3258c
  GPU: a5d6f56b6536c6288791811b9abfcfde
```

All still fail from byte 0.

### Components Verified Working
✅ Blake2b
✅ Argon2 H'
✅ VM initialization
✅ loop_counter encoding (now fixed)

### Remaining Suspects
❌ Instruction execution loop
❌ Program generation/shuffling  
❌ execute_one_instruction implementation
❌ ip field management

