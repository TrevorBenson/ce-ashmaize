# Opcode Comparison: CPU vs GPU

## CPU (from src/b2.rs)

```
0..40    (0-39)     => Add
40..80   (40-79)    => Mul
80..96   (80-95)    => MulH
96..112  (96-111)   => Div
112..128 (112-127)  => Mod
128..138 (128-137)  => ISqrt
138..148 (138-147)  => BitRev
148..188 (148-187)  => Xor
188..204 (188-203)  => RotL
204..220 (204-219)  => RotR
220..240 (220-239)  => Neg
240..248 (240-247)  => And
248..=255(248-255)  => Hash
```

## GPU (from cuda/ashmaize.cu)

```
< 40     (0-39)     => Add          ✓
< 80     (40-79)    => Mul          ✓
< 96     (80-95)    => MulH         ✓
< 112    (96-111)   => Div          ✓
< 128    (112-127)  => Mod          ✓ FIXED (was Div)
< 138    (128-137)  => ISqrt        ✓
< 148    (138-147)  => BitRev       ✓
< 188    (148-187)  => Xor          ✓
< 204    (188-203)  => RotL         ✓
< 220    (204-219)  => RotR         ✓
< 240    (220-239)  => Neg          ✓
< 248    (240-247)  => And          ✓
else     (248-255)  => Blake2b Hash ✓
```

All ranges match!

## Fixes Applied

1. loop_counter encoding: 8 bytes → 4 bytes ✓
2. Mod operator: `/` → `%` ✓

## Remaining Issues

Hash still doesn't match after both fixes.
Results have changed, indicating progress:
- Before fixes: 62a2f31a42c086f8
- After loop_counter fix: 55d4c9663bd73ce7
- After modulo fix: bc3f7a5a3cbd5e68

CPU target: 0ec51a39c5ba9d33

