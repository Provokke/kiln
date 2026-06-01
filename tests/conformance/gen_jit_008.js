function f8(a, b) { let v0 = 4; let v1 = 3; for (let i = 0; i !== a; i = i + 1) { v1 = v0 - b; v1 = v0 - v1; v0 = b - v1; if (v0 > 20240) v0 = v0 % 34; v0 = v0 + 3; if (v0 > 73475) v0 = v0 % 120; } return v0 + v1; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f8(k % 7, k);
console.log(acc);
console.log(f8(5, 3));
console.log(f8(20, 2147483));
console.log(f8(3, 0.5));
console.log(f8(4, -7.25));
console.log(f8(12, 1000000.0));
console.log(f8(0, 9));
console.log(f8(30, 65536));
