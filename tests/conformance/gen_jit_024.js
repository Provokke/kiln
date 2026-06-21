function f24(a, b) { let v0 = 5; let v1 = 2; let v2 = 3; for (let i = 0; i < a; i = i + 1) { v2 = v1 * v0; if (v2 > 97950) v2 = v2 % 999; v2 = v2 - v0; v1 = v0 - b; v1 = b * 3; if (v1 > 18899) v1 = v1 % 51; } return v0 + v1 + v2; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f24(k % 7, k);
console.log(acc);
console.log(f24(5, 3));
console.log(f24(20, 2147483));
console.log(f24(3, 0.5));
console.log(f24(4, -7.25));
console.log(f24(12, 1000000.0));
console.log(f24(0, 9));
console.log(f24(30, 65536));
