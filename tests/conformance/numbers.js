console.log(0.1 + 0.2, 1 / 3, 1e21, 1e-7, 123456789012345680000, 2 ** 53, -0, 0 / 0, 1 / 0, -1 / 0);
console.log(7 % 3, -7 % 3, 7.5 % 2, 2 ** -1, 10 / 4, (0.1).toFixed(3), (255).toString(16), (255).toString(2));
console.log(5 & 3, 5 | 3, 5 ^ 3, ~5, 1 << 31, -16 >> 2, -16 >>> 28, 2 ** 32 | 0);
console.log(parseInt("42px"), parseInt("0x1f"), parseInt("z", 36), parseFloat("3.14abc"), Number("12"), Number(""), Number("x"));
console.log(isNaN("abc"), isFinite(1 / 0), Number.isInteger(5.0), Number.MAX_SAFE_INTEGER);
console.log(1000000 * 1000000, 123.456, 100, 1.5e300 * 1e10, 5e-324);
