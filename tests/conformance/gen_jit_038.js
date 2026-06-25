function f38(a, b) { let v0 = 4; let v1 = 2; let v2 = 4; for (let i = 0; i <= a; i = i + 1) { v0 = b * v1; v2 = v2 + v2; if (v2 > 80013) v2 = v2 % 744; v0 = v0 * v0; } return v0 + v1 + v2; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f38(k % 7, k);
console.log(acc);
console.log(f38(5, 3));
console.log(f38(20, 2147483));
console.log(f38(3, 0.5));
console.log(f38(4, -7.25));
console.log(f38(12, 1000000.0));
console.log(f38(0, 9));
console.log(f38(30, 65536));
