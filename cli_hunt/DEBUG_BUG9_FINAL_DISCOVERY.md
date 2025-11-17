# Bug #9: Final Discovery - Blake2b Hash Mismatch

## Critical Finding

After extensive tracing, we discovered:

### What Matches (100%)
1. ✓ lit1 = `0x57aaaa9e896b78ed`
2. ✓ lit2 = `0x9c6e1a4905d7e8b0`
3. ✓ src1 (literal) = `0x57aaaa9e896b78ed`  
4. ✓ src2 (memory) = `0xe8bcb6feb065ea96`
5. ✓ Blake2b input = `ed786b899eaaaa5796ea65b0feb6bce8`

### Results
- **GPU**: `0x5694e7110ab9f4d3`
- **CPU**: `0x5555698eee0fbaef`
- **Correct (b2sum)**: `0x5694e7110ab9f4d3`

**THE GPU IS CORRECT!**

## Verification

```bash
$ echo -n "ed786b899eaaaa5796ea65b0feb6bce8" | xxd -r -p | b2sum -l 512
1382edd60a4e92f3b34b09753515e447d3f4b90a11e794565c4ca3e7708093c4c35aa631325029c138c5b4eb03c7e9887262082e9a06a6f801a1083a7628dd4d
```

Chunk 2 (bytes 16-23): `d3f4b90a11e79456`  
Little-endian u64: `0x5694e7110ab9f4d3` ✓ MATCHES GPU

## Mystery

The CPU implementation is pristine (verified against commit 7d40de577866776da1c3ebb15392cb61d70db0ef).

If the CPU code is correct but produces wrong results, then either:
1. The CPU's trace is wrong (we extracted wrong values)
2. The CPU's Blake2b library (`blake2` crate) is buggy
3. The CPU is actually getting DIFFERENT src2 value than we traced

## Next Step

Need to instrument the ACTUAL CPU VM execution to print:
- The actual src1 value used
- The actual src2 value used  
- The actual Blake2b input bytes
- The actual Blake2b output

Then compare with GPU to find the REAL discrepancy.

