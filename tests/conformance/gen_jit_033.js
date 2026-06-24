function f33(a, b) { let v0 = 5; let v1 = 5; let v2 = 2; for (let i = 0; i !== a; i = i + 1) { v1 = v1 * a; if (v1 > 62983) v1 = v1 % 929; v2 = v1 * v1; v2 = 8 + v2; if (v2 > 1484) v2 = v2 % 529; v2 = v1 * v2; if (v2 > 81607) v2 = v2 % 594; } return v0 + v1 + v2; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f33(k % 7, k);
console.log(acc);
console.log(f33(5, 3));
console.log(f33(20, 2147483));
console.log(f33(3, 0.5));
console.log(f33(4, -7.25));
console.log(f33(12, 1000000.0));
console.log(f33(0, 9));
console.log(f33(30, 65536));
