function f57(a, b) { let v0 = 0; let v1 = 4; let v2 = 1; for (let i = 0; i !== a; i = i + 1) { v1 = v2 * v2; v1 = v1 * 4; v2 = v0 * b; if (v2 > 89818) v2 = v2 % 583; v2 = v0 * b; v1 = v2 - b; if (v1 > 2853) v1 = v1 % 948; } return v0 + v1 + v2; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f57(k % 7, k);
console.log(acc);
console.log(f57(5, 3));
console.log(f57(20, 2147483));
console.log(f57(3, 0.5));
console.log(f57(4, -7.25));
console.log(f57(12, 1000000.0));
console.log(f57(0, 9));
console.log(f57(30, 65536));
