function f39(a, b) { let v0 = 0; let v1 = 3; let v2 = 0; for (let i = 0; i <= a; i = i + 1) { v0 = v1 + a; v0 = 5 + v0; } return v0 + v1 + v2; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f39(k % 7, k);
console.log(acc);
console.log(f39(5, 3));
console.log(f39(20, 2147483));
console.log(f39(3, 0.5));
console.log(f39(4, -7.25));
console.log(f39(12, 1000000.0));
console.log(f39(0, 9));
console.log(f39(30, 65536));
