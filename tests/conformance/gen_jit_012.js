function f12(a, b) { let v0 = 2; let v1 = 3; let v2 = 2; for (let i = 0; i !== a; i = i + 1) { v0 = b - v2; if (v0 > 92741) v0 = v0 % 609; v2 = v0 - b; v1 = b - v1; } return v0 + v1 + v2; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f12(k % 7, k);
console.log(acc);
console.log(f12(5, 3));
console.log(f12(20, 2147483));
console.log(f12(3, 0.5));
console.log(f12(4, -7.25));
console.log(f12(12, 1000000.0));
console.log(f12(0, 9));
console.log(f12(30, 65536));
