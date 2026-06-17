function f20(a, b) { let v0 = 4; let v1 = 2; for (let i = 0; i < a; i = i + 1) { v0 = a + a; v0 = 6 - v0; v0 = b * 7; v1 = b + v0; if (v1 > 43671) v1 = v1 % 72; } return v0 + v1; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f20(k % 7, k);
console.log(acc);
console.log(f20(5, 3));
console.log(f20(20, 2147483));
console.log(f20(3, 0.5));
console.log(f20(4, -7.25));
console.log(f20(12, 1000000.0));
console.log(f20(0, 9));
console.log(f20(30, 65536));
