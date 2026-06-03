function f11(a, b) { let v0 = 1; let v1 = 5; let v2 = 2; for (let i = 0; i < a; i = i + 1) { v0 = v0 * v2; if (v0 > 72130) v0 = v0 % 926; v0 = v0 * v2; v2 = v0 * a; v2 = v0 - v0; } return v0 + v1 + v2; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f11(k % 7, k);
console.log(acc);
console.log(f11(5, 3));
console.log(f11(20, 2147483));
console.log(f11(3, 0.5));
console.log(f11(4, -7.25));
console.log(f11(12, 1000000.0));
console.log(f11(0, 9));
console.log(f11(30, 65536));
