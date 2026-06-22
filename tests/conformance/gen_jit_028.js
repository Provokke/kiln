function f28(a, b) { let v0 = 2; let v1 = 1; for (let i = 0; i !== a; i = i + 1) { v1 = 8 + 7; v0 = v1 + v1; if (v0 > 54169) v0 = v0 % 255; v1 = b + v0; if (v1 > 6474) v1 = v1 % 73; v1 = a * v1; v1 = v0 * v0; if (v1 > 91006) v1 = v1 % 545; } return v0 + v1; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f28(k % 7, k);
console.log(acc);
console.log(f28(5, 3));
console.log(f28(20, 2147483));
console.log(f28(3, 0.5));
console.log(f28(4, -7.25));
console.log(f28(12, 1000000.0));
console.log(f28(0, 9));
console.log(f28(30, 65536));
