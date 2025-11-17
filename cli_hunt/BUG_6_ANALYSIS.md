# Bug #6 Analysis - prog_digest Update

## Bug Description
GPU's `execute_one_instruction` was missing the prog_digest update at the end.

## Fix Applied
Added at end of execute_one_instruction:
```cuda
blake2b_update(vm.prog_digest_state, prog_chunk, INSTR_SIZE);
```

## Result
- Hash changed from 83b757de... to 83e2b503...
- Still doesn't match CPU (0ec51a39...)
- Fix had an effect (output changed) but didn't solve the problem

## Status
Partial fix - there are more bugs remaining.

## Continuing Investigation
Need to verify ROM access calculation between CPU and GPU.

