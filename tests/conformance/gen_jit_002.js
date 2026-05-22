function f2(a, b) { let v0 = 4; let v1 = 5; for (let i = 0; i < a; i = i + 1) { v0 = v0 * v1; if (v0 > 19755) v0 = v0 % 273; v1 = v0 + 3; v0 = a * v1; } return v0 + v1; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f2(k % 7, k);
console.log(acc);
console.log(f2(5, 3));
console.log(f2(20, 2147483));
console.log(f2(3, 0.5));
console.log(f2(4, -7.25));
console.log(f2(12, 1000000.0));
console.log(f2(0, 9));
console.log(f2(30, 65536));
