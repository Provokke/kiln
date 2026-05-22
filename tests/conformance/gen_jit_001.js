function f1(a, b) { let v0 = 0; let v1 = 3; for (let i = 0; i !== a; i = i + 1) { v0 = 3 + 9; v1 = b + a; v1 = v0 * b; if (v1 > 291) v1 = v1 % 78; v0 = b - v1; } return v0 + v1; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f1(k % 7, k);
console.log(acc);
console.log(f1(5, 3));
console.log(f1(20, 2147483));
console.log(f1(3, 0.5));
console.log(f1(4, -7.25));
console.log(f1(12, 1000000.0));
console.log(f1(0, 9));
console.log(f1(30, 65536));
